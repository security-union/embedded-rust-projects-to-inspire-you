use gpio_cdev::{Chip, LineRequestFlags};
use std::time::Instant;

const GPIO13: u32 = 6;
const PWM_FREQUENCY_HZ: f64 = 50.0; // 50 Hz corresponds to a 20 ms period
const PERIOD_MS: u64 = 20; // 20 ms period
const PULSE_MIN_US: u64 = 1200; // Minimum pulse width
const PULSE_NEUTRAL_US: u64 = 1500; // Neutral pulse width
const PULSE_MAX_US: u64 = 1800; // Maximum pulse width
const CYCLE_DURATION_SECONDS: u64 = 20;

/**
 * This program will smoothly change the pulse width of a servo from PULSE_MIN_US to PULSE_MAX_US over 20 seconds.
 */
fn main() -> Result<(), gpio_cdev::Error> {
    // Open the GPIO chip; it's gpiochip0 for the main GPIO controller on a Raspberry Pi
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line = chip.get_line(GPIO13)?;
    let line = line.request(LineRequestFlags::OUTPUT, 0, "pwm")?;
    loop {
        let start_time = Instant::now();

        loop {
            let elapsed_time = start_time.elapsed().as_secs_f64();
            if elapsed_time >= CYCLE_DURATION_SECONDS as f64 {
                break;
            }

            // Calculate the pulse width based on the elapsed time
            let pulse_width_us = PULSE_MIN_US as f64
                + (elapsed_time / CYCLE_DURATION_SECONDS as f64)
                    * (PULSE_MAX_US as f64 - PULSE_MIN_US as f64);
            let time_on_us = pulse_width_us;
            let time_off_us = (PERIOD_MS * 1000) as f64 - time_on_us;

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

    Ok(())
}
