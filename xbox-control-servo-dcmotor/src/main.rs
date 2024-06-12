use gilrs::{Axis, Event, EventType::*, Gilrs};
use gpio_cdev::{Chip, LineRequestFlags};
use std::panic;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

const SERVO_GPIO: u32 = 6;
const DC_MOTOR_GPIO_1: u32 = 17;
const DC_MOTOR_GPIO_2: u32 = 27;
const DC_MOTOR_PERIOD_MS: u64 = 10;
const SERVO_PERIOD_US: f32 = 20000f32;
const PULSE_MIN_US: f32 = 1200f32; // Minimum pulse width in microseconds
const PULSE_NEUTRAL_US: f32 = 1500f32; // Neutral pulse width in microseconds
const PULSE_MAX_US: f32 = 1800f32; // Maximum pulse width in microseconds
const DEADZONE: f32 = 100f32; // Deadzone threshold for the left stick's Y-axis (0.1 scaled to 1000)
const RAMP_UP_INCREMENT: u64 = 10; // Incremental step for ramping up the speed

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

fn servo_control_thread(receiver: Receiver<f32>) -> Result<(), gpio_cdev::Error> {
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line = chip.get_line(SERVO_GPIO)?;
    let line = line.request(LineRequestFlags::OUTPUT, 0, "pwm")?;

    let mut time_on_us = PULSE_NEUTRAL_US;
    let pulse_range = PULSE_MAX_US - PULSE_MIN_US;

    loop {
        while let Ok(value) = receiver.try_recv() {
            // value goes from -1 to 1, so we need to scale it to the pulse range
            // and add the minimum pulse width
            let scaled_value = ((value + 1.0) / 2.0) * pulse_range;
            time_on_us = PULSE_MIN_US + scaled_value;
        }

        let time_off_us = SERVO_PERIOD_US  - time_on_us;
        line.set_value(1)?;
        thread::sleep(Duration::from_micros(time_on_us as u64));
        line.set_value(0)?;
        thread::sleep(Duration::from_micros(time_off_us as u64));
    }
}

fn motor_control_thread(receiver: Receiver<f32>) -> Result<(), gpio_cdev::Error> {
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line1 = chip.get_line(DC_MOTOR_GPIO_1)?;
    let line1 = line1.request(LineRequestFlags::OUTPUT, 0, "motor1")?;
    let line2 = chip.get_line(DC_MOTOR_GPIO_2)?;
    let line2 = line2.request(LineRequestFlags::OUTPUT, 0, "motor2")?;

    let mut current_duty_cycle = 0;
    let mut target_duty_cycle = 0;
    let mut last_direction = Direction::Forward;

    loop {
        while let Ok(value) = receiver.try_recv() {
            target_duty_cycle = if (value * 1000.0).abs() < DEADZONE {
                0
            } else {
                (value * 1000.0).abs() as u64
            };
            last_direction = if value > 0.0 {
                Direction::Forward
            } else {
                Direction::Backward
            };
        }

        if current_duty_cycle != target_duty_cycle {
            // Ramp up or down to the target duty cycle
            if current_duty_cycle < target_duty_cycle {
                current_duty_cycle += RAMP_UP_INCREMENT;
                if current_duty_cycle > target_duty_cycle {
                    current_duty_cycle = target_duty_cycle;
                }
            } else {
                current_duty_cycle -= RAMP_UP_INCREMENT;
                if current_duty_cycle < target_duty_cycle {
                    current_duty_cycle = target_duty_cycle;
                }
            }

            if target_duty_cycle == 0 {
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
        let time_on_ms = (current_duty_cycle * DC_MOTOR_PERIOD_MS) / 1000;
        let time_off_ms = DC_MOTOR_PERIOD_MS - time_on_ms;

        thread::sleep(Duration::from_millis(time_on_ms));
        // Stop the motor by setting both lines to low
        line1.set_value(0)?;
        line2.set_value(0)?;
        thread::sleep(Duration::from_millis(time_off_ms));
    }
}

fn controller_read_loop(
    mut gilrs: Gilrs,
    servo_tx: Sender<f32>,
    motor_tx: Sender<f32>,
) -> Result<(), gpio_cdev::Error> {
    loop {
        while let Some(Event { event, .. }) = gilrs.next_event() {
            match event {
                AxisChanged(Axis::RightStickX, value, _) => {
                    servo_tx.send(value).unwrap();
                }
                AxisChanged(Axis::LeftStickY, value, _) => {
                    motor_tx.send(value).unwrap();
                }
                _ => {}
            }
        }
    }
}
