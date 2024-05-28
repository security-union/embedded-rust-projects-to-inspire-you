use gilrs::{Axis, Event, Gilrs, EventType::*, EventType };
use gpio_cdev::{Chip, LineRequestFlags};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

const GPIO13: u32 = 6;
const PWM_FREQUENCY_HZ: f64 = 50.0; // 50 Hz corresponds to a 20 ms period
const PERIOD_MS: f32 = 20.0; // 20 ms period
const PULSE_MIN_US: f32 = 1200.0; // Minimum pulse width
const PULSE_NEUTRAL_US: f32 = 1500.0; // Neutral pulse width
const PULSE_MAX_US: f32 = 1800.0; // Maximum pulse width

fn main() -> Result<(), gpio_cdev::Error> {
    let mut gilrs = Gilrs::new().unwrap();

    // Iterate over all connected gamepads
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let pulse_width = Arc::new(Mutex::new(PULSE_NEUTRAL_US));
    let pulse_width_clone = Arc::clone(&pulse_width);

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

    loop {
        // Get the state of the right stick's horizontal axis
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            println!("{:?} New event from {}: {:?}", time, id, event);
            match event {
                // Determine if the event is right stick's horizontal axis
                AxisChanged(axis, value, _) => {
                    if axis == Axis::RightStickX {
                        let new_pulse_width = PULSE_MIN_US
                            + ((value + 1.0) / 2.0) * (PULSE_MAX_US - PULSE_MIN_US);

                        {
                            let mut pw = pulse_width.lock().unwrap();
                            *pw = new_pulse_width;
                        }
                    }
                }
                _ => (),
            }
        }
        // Determine if the event is right stick's horizontal axis
    }
}