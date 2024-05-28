use gilrs::{Axis, Event, EventType::*, Gilrs};
use gpio_cdev::{Chip, LineRequestFlags};
use std::panic;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const GPIO13: u32 = 6;
const GPIO17: u32 = 17;
const GPIO27: u32 = 27;
const MOTOR_PERIOD_MS: i32 = 10; // 20 ms period
const SERVO_PERIOD_MS: f32 = 20.0; // 20 ms period for the servo
const PULSE_MIN_US: f32 = 1200.0; // Minimum pulse width
const PULSE_NEUTRAL_US: f32 = 1500.0; // Neutral pulse width
const PULSE_MAX_US: f32 = 1800.0; // Maximum pulse width
const DEADZONE: f32 = 100.0; // Deadzone threshold for the left stick's Y-axis
const RAMP_UP_INCREMENT: i32 = 10; // Incremental step for ramping up the speed

fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        eprintln!("Panic occurred: {:?}", panic_info);
        std::process::abort();
    }));
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Forward,
    Backward,
}

fn main() -> Result<(), gpio_cdev::Error> {
    set_panic_hook();
    let gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let pulse_width = Arc::new(Mutex::new(PULSE_NEUTRAL_US));
    let motor_control = Arc::new(Mutex::new((0i32, 0i32, Direction::Forward))); // (current_duty_cycle, target_duty_cycle, direction)

    {
        let pulse_width = pulse_width.clone();
        thread::spawn(move || pwm_thread(pulse_width.clone()));
    }
    {
        let motor_control = motor_control.clone();
        thread::spawn(move || motor_control_thread(motor_control.clone()));
    }
    event_loop(gilrs, pulse_width.clone(), motor_control.clone());

    Ok(())
}

fn pwm_thread(pulse_width: Arc<Mutex<f32>>) {
    let mut chip = Chip::new("/dev/gpiochip0").unwrap();
    let line = chip.get_line(GPIO13).unwrap();
    let line = line.request(LineRequestFlags::OUTPUT, 0, "pwm").unwrap();

    loop {
        let time_on_us;
        {
            let pw = pulse_width.lock().unwrap();
            time_on_us = *pw;
        }
        let time_off_us = (SERVO_PERIOD_MS * 1000f32) - time_on_us;

        line.set_value(1).unwrap();
        thread::sleep(Duration::from_micros(time_on_us as u64));
        line.set_value(0).unwrap();
        thread::sleep(Duration::from_micros(time_off_us as u64));
    }
}

fn motor_control_thread(
    motor_control: Arc<Mutex<(i32, i32, Direction)>>,
) -> Result<(), gpio_cdev::Error> {
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line1 = chip.get_line(GPIO17)?;
    let line1 = line1.request(LineRequestFlags::OUTPUT, 0, "motor1")?;
    let line2 = chip.get_line(GPIO27)?;
    let line2 = line2.request(LineRequestFlags::OUTPUT, 0, "motor2")?;

    loop {
        let (mut current_duty_cycle, target_duty_cycle, direction);
        {
            let mc = motor_control.lock().unwrap();
            current_duty_cycle = mc.0;
            target_duty_cycle = mc.1;
            direction = mc.2;
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

            {
                let mut mc = motor_control.lock().unwrap();
                mc.0 = current_duty_cycle;
            }

            // thread::sleep(Duration::from_millis(RAMP_UP_INTERVAL_MS));
        }

        if current_duty_cycle > 0 {
            // Set the direction of the motor
            if direction == Direction::Forward {
                line1.set_value(1)?;
                line2.set_value(0)?;
            } else {
                line1.set_value(0)?;
                line2.set_value(1)?;
            }

            // PWM control for the motor speed
            let time_on_ms = (current_duty_cycle * MOTOR_PERIOD_MS) / 1000;
            let time_off_ms = MOTOR_PERIOD_MS - time_on_ms;
            println!(
                "current_duty_cycle: {} target_duty_cycle {}",
                current_duty_cycle, target_duty_cycle
            );
            println!("time_on_ms: {}, time_off_ms: {}", time_on_ms, time_off_ms);

            thread::sleep(Duration::from_millis(time_on_ms as u64));
            line1.set_value(0)?;
            line2.set_value(0)?;
            thread::sleep(Duration::from_millis(time_off_ms as u64));
        } else {
            // Stop the motor by setting both lines to low
            line1.set_value(0)?;
            line2.set_value(0)?;
            thread::sleep(Duration::from_millis(MOTOR_PERIOD_MS as u64));
        }
    }
}

fn event_loop(
    mut gilrs: Gilrs,
    pulse_width: Arc<Mutex<f32>>,
    motor_control: Arc<Mutex<(i32, i32, Direction)>>,
) -> Result<(), gpio_cdev::Error> {
    loop {
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            // println!("{:?} New event from {}: {:?}", time, id, event);
            match event {
                AxisChanged(axis, value, _) => {
                    if axis == Axis::RightStickX {
                        let new_pulse_width =
                            PULSE_MIN_US + ((value + 1.0) / 2.0) * (PULSE_MAX_US - PULSE_MIN_US);

                        {
                            let mut pw = pulse_width.lock().unwrap();
                            *pw = new_pulse_width;
                        }
                    }
                    if axis == Axis::LeftStickY {
                        let target_duty_cycle = if (value * 1000.0).abs() < DEADZONE {
                            0
                        } else {
                            (value * 1000.0).abs() as i32
                        };

                        let direction = if value > 0.0 {
                            Direction::Forward
                        } else {
                            Direction::Backward
                        };

                        {
                            let mut mc = motor_control.lock().unwrap();
                            mc.1 = target_duty_cycle;
                            mc.2 = direction;
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
