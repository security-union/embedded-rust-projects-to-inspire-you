use gilrs::{Axis, Event, EventType::*, Gilrs};
use gpio_cdev::{Chip, LineRequestFlags};
use std::panic;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

const GPIO13: u32 = 6;
const GPIO17: u32 = 17;
const GPIO27: u32 = 27;
const PERIOD_MS: i32 = 10; // 20 ms period
const SERVO_PERIOD_MS: i32 = 20;
const PULSE_MIN_US: i32 = 1200; // Minimum pulse width in microseconds
const PULSE_NEUTRAL_US: i32 = 1500; // Neutral pulse width in microseconds
const PULSE_MAX_US: i32 = 1800; // Maximum pulse width in microseconds
const DEADZONE: i32 = 100; // Deadzone threshold for the left stick's Y-axis (0.1 scaled to 1000)
const RAMP_UP_INCREMENT: i32 = 10; // Incremental step for ramping up the speed

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Forward,
    Backward,
}

fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        eprintln!("Panic occurred: {:?}", panic_info);
        std::process::abort();
    }));
}

fn main() -> Result<(), gpio_cdev::Error> {
    set_panic_hook(); // Set the global panic hook

    let gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let (servo_tx, servo_rx) = mpsc::channel();
    let (motor_tx, motor_rx) = mpsc::channel();

    let pwm_thread_handle = thread::spawn(move || {
        servo_control_thread(servo_rx).expect("PWM thread panicked");
    });

    let motor_control_thread_handle = thread::spawn(move || {
        motor_control_thread(motor_rx).expect("Motor control thread panicked");
    });

    controller_read_loop(gilrs, servo_tx, motor_tx).expect("Event loop panicked");

    pwm_thread_handle.join().expect("PWM thread panicked");
    motor_control_thread_handle
        .join()
        .expect("Motor control thread panicked");

    Ok(())
}

fn servo_control_thread(receiver: Receiver<i32>) -> Result<(), gpio_cdev::Error> {
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line = chip.get_line(GPIO13)?;
    let line = line.request(LineRequestFlags::OUTPUT, 0, "pwm")?;

    let mut last_pulse_width = PULSE_NEUTRAL_US;

    loop {
        while let Ok(time_on_us) = receiver.try_recv() {
            last_pulse_width = time_on_us;
        }

        let time_off_us = SERVO_PERIOD_MS * 1000 - last_pulse_width;
        println!(
            "Pulse width: {} us time off {}",
            last_pulse_width, time_off_us
        );
        line.set_value(1)?;
        thread::sleep(Duration::from_micros(last_pulse_width as u64));
        line.set_value(0)?;
        thread::sleep(Duration::from_micros(time_off_us as u64));
    }
}

fn motor_control_thread(receiver: Receiver<(i32, Direction)>) -> Result<(), gpio_cdev::Error> {
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line1 = chip.get_line(GPIO17)?;
    let line1 = line1.request(LineRequestFlags::OUTPUT, 0, "motor1")?;
    let line2 = chip.get_line(GPIO27)?;
    let line2 = line2.request(LineRequestFlags::OUTPUT, 0, "motor2")?;

    let mut current_duty_cycle = 0;
    let mut last_duty_cycle = 0;
    let mut last_direction = Direction::Forward;

    loop {
        while let Ok((target_duty_cycle, direction)) = receiver.try_recv() {
            last_duty_cycle = target_duty_cycle;
            last_direction = direction;
        }

        if current_duty_cycle != last_duty_cycle {
            // Ramp up or down to the target duty cycle
            if current_duty_cycle < last_duty_cycle {
                current_duty_cycle += RAMP_UP_INCREMENT;
                if current_duty_cycle > last_duty_cycle {
                    current_duty_cycle = last_duty_cycle;
                }
            } else {
                current_duty_cycle -= RAMP_UP_INCREMENT;
                if current_duty_cycle < last_duty_cycle {
                    current_duty_cycle = last_duty_cycle;
                }
            }

            if last_duty_cycle == 0 {
                current_duty_cycle = 0;
            }
        }
        if last_direction == Direction::Forward {
            line1.set_value(1)?;
            line2.set_value(0)?;
        } else {
            line1.set_value(0)?;
            line2.set_value(1)?;
        }

        // PWM control for the motor speed
        let time_on_ms = (current_duty_cycle * PERIOD_MS as i32) / 1000;
        let time_off_ms = PERIOD_MS as i32 - time_on_ms;

        thread::sleep(Duration::from_millis(time_on_ms as u64));
        // Stop the motor by setting both lines to low
        line1.set_value(0)?;
        line2.set_value(0)?;
        thread::sleep(Duration::from_millis(time_off_ms as u64));
    }
}

fn controller_read_loop(
    mut gilrs: Gilrs,
    servo_tx: Sender<i32>,
    motor_tx: Sender<(i32, Direction)>,
) -> Result<(), gpio_cdev::Error> {
    loop {
        while let Some(Event { event, .. }) = gilrs.next_event() {
            // println!("{:?} New event from {}: {:?}", time, id, event);
            match event {
                AxisChanged(axis, value, _) => {
                    if axis == Axis::RightStickX {
                        let scaled_value =
                            ((value + 1.0) / 2.0 * (PULSE_MAX_US - PULSE_MIN_US) as f32) as i32;
                        let new_pulse_width = PULSE_MIN_US + scaled_value;
                        servo_tx.send(new_pulse_width).unwrap();
                    }

                    if axis == Axis::LeftStickY {
                        let target_duty_cycle = if (value * 1000.0).abs() < DEADZONE as f32 {
                            0
                        } else {
                            (value * 1000.0).abs() as i32
                        };
                        let direction = if value > 0.0 {
                            Direction::Forward
                        } else {
                            Direction::Backward
                        };
                        motor_tx.send((target_duty_cycle, direction)).unwrap();
                    }
                }
                _ => (),
            }
        }
    }
}
