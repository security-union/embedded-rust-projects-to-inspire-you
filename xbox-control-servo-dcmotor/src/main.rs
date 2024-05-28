use gilrs::{Axis, Event, EventType::*, Gilrs};
use gpio_cdev::{Chip, LineRequestFlags};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const GPIO13: u32 = 6;
const GPIO17: u32 = 17;
const GPIO27: u32 = 27;
const PERIOD_MS: f32 = 20.0; // 20 ms period
const PULSE_MIN_US: f32 = 1200.0; // Minimum pulse width
const PULSE_NEUTRAL_US: f32 = 1500.0; // Neutral pulse width
const PULSE_MAX_US: f32 = 1800.0; // Maximum pulse width
const DEADZONE: f32 = 0.1; // Deadzone threshold for the left stick's Y-axis

fn main() -> Result<(), gpio_cdev::Error> {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let pulse_width = Arc::new(Mutex::new(PULSE_NEUTRAL_US));
    let pulse_width_clone = Arc::clone(&pulse_width);

    let motor_control = Arc::new(Mutex::new((0.0, 0.0))); // (duty_cycle, direction)
    let motor_control_clone = Arc::clone(&motor_control);

    thread::spawn(move || {
        let mut chip = Chip::new("/dev/gpiochip0").unwrap();
        let line = chip.get_line(GPIO13).unwrap();
        let line = line.request(LineRequestFlags::OUTPUT, 0, "pwm").unwrap();

        loop {
            let time_on_us;
            {
                let pw = pulse_width_clone.lock().unwrap();
                time_on_us = *pw;
            }
            let time_off_us = (PERIOD_MS * 1000f32) - time_on_us;

            line.set_value(1).unwrap();
            thread::sleep(Duration::from_micros(time_on_us as u64));
            line.set_value(0).unwrap();
            thread::sleep(Duration::from_micros(time_off_us as u64));
        }
    });

    thread::spawn(move || {
        let mut chip = Chip::new("/dev/gpiochip0").unwrap();
        let line1 = chip.get_line(GPIO17).unwrap();
        let line1 = line1
            .request(LineRequestFlags::OUTPUT, 0, "motor1")
            .unwrap();
        let line2 = chip.get_line(GPIO27).unwrap();
        let line2 = line2
            .request(LineRequestFlags::OUTPUT, 0, "motor2")
            .unwrap();

        loop {
            let (duty_cycle, direction);
            {
                let mc = motor_control_clone.lock().unwrap();
                duty_cycle = mc.0;
                direction = mc.1;
            }

            if duty_cycle > 0.0 {
                // Set the direction of the motor
                line1
                    .set_value(if direction > 0.0 { 1 } else { 0 })
                    .unwrap();
                line2
                    .set_value(if direction < 0.0 { 1 } else { 0 })
                    .unwrap();

                // PWM control for the motor speed
                let time_on_ms = (duty_cycle * PERIOD_MS) as u64;
                let time_off_ms = (PERIOD_MS - duty_cycle * PERIOD_MS) as u64;

                thread::sleep(Duration::from_millis(time_on_ms));
                thread::sleep(Duration::from_millis(time_off_ms));
            } else {
                // Stop the motor by setting both lines to low
                line1.set_value(0).unwrap();
                line2.set_value(0).unwrap();
                thread::sleep(Duration::from_millis(PERIOD_MS as u64));
            }
        }
    });

    loop {
        // Get the state of the right stick's horizontal axis
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            println!("{:?} New event from {}: {:?}", time, id, event);
            match event {
                // Determine if the event is right stick's horizontal axis
                AxisChanged(axis, value, _) => {
                    if axis == Axis::RightStickX {
                        let new_pulse_width =
                            PULSE_MIN_US + ((value + 1.0) / 2.0) * (PULSE_MAX_US - PULSE_MIN_US);

                        {
                            let mut pw = pulse_width.lock().unwrap();
                            *pw = new_pulse_width;
                        }
                    }

                    // Handle left stick's vertical axis
                    if axis == Axis::LeftStickY {
                        let duty_cycle = if value.abs() < DEADZONE {
                            0.0
                        } else {
                            value.abs()
                        };
                        let direction = if value.abs() < DEADZONE {
                            0.0
                        } else {
                            value.signum()
                        };

                        {
                            let mut mc = motor_control.lock().unwrap();
                            mc.0 = duty_cycle;
                            mc.1 = direction;
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
