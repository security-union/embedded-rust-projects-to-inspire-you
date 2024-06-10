use gpio_cdev::{Chip, LineRequestFlags};
use std::time::Instant;

const GPIO13: u32 = 6;
const PERIOD_US: f64 = 20.0; // 20 ms period
const PULSE_MIN_US: f64 = 1200.0; // Minimum pulse width
const PULSE_MAX_US: f64 = 1800.0; // Maximum pulse width
const CYCLE_DURATION_SECS: f64 = 10.0;

/**
 * This program will smoothly change the pulse width of a servo from PULSE_MIN_US to PULSE_MAX_US over CYCLE_DURATION_SECS seconds.
 */
fn main() -> Result<(), gpio_cdev::Error> {
    // Open the GPIO chip; it's gpiochip0 for the main GPIO controller on a Raspberry Pi
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line = chip.get_line(GPIO13)?;
    let line = line.request(LineRequestFlags::OUTPUT, 0, "pwm")?;
    loop {
        let start_time: Instant = Instant::now();
        loop {
            let elapsed_time = start_time.elapsed().as_secs_f64();
            if elapsed_time >= CYCLE_DURATION_SECS {
                break;
            }
            // Calculate the pulse width based on the elapsed time
            let pulse_width_us: f64 = PULSE_MIN_US
                + (elapsed_time / CYCLE_DURATION_SECS)
                    * (PULSE_MAX_US - PULSE_MIN_US);
            let time_on_us = pulse_width_us;
            let time_off_us = PERIOD_US - time_on_us;

            println!(
            "Elapsed time: {:.2} s, Pulse width: {:.2} µs, Time on: {:.2} µs, Time off: {:.2} µs",
            elapsed_time, pulse_width_us, time_on_us, time_off_us
        );

            line.set_value(1)?;
            std::thread::sleep(std::time::Duration::from_micros(time_on_us as u64));
            line.set_value(0)?;
            std::thread::sleep(std::time::Duration::from_micros(time_off_us as u64));
        }
    }
}
