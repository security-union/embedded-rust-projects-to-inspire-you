use gpio_cdev::{Chip, LineRequestFlags};
use std::time::Instant;

const GPIO6: u32 = 6;
const PERIOD_US: f64 = 20000f64; // 20 ms period
const PULSE_MIN_US: f64 = 1200.0; // Minimum pulse width
const PULSE_MAX_US: f64 = 1800.0; // Maximum pulse width
const CYCLE_DURATION_SECS: f64 = 5.0;

/**
 * This program will smoothly change the pulse width of a servo from PULSE_MIN_US to PULSE_MAX_US over CYCLE_DURATION_SECS seconds.
 */
fn main() -> Result<(), gpio_cdev::Error> {
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line = chip.get_line(GPIO6)?;
    let line = line.request(LineRequestFlags::OUTPUT, 0, "pwm")?;
    let pulse_delta = PULSE_MAX_US -  PULSE_MIN_US;
    loop {
        let start_time = Instant::now();
        loop {
            let elapsed_time = start_time.elapsed().as_secs_f64();
            if elapsed_time > CYCLE_DURATION_SECS {
                break;
            }
            let time_on_us_increment = (elapsed_time / CYCLE_DURATION_SECS) * pulse_delta;
            let time_on_us: f64 = PULSE_MIN_US + time_on_us_increment;
            let time_off_us = PERIOD_US - time_on_us;
            line.set_value(1)?;

            println!("elapsed_time {} time_on_us: {} time_off_us: {}", elapsed_time, time_on_us, time_off_us);
            std::thread::sleep(std::time::Duration::from_micros(time_on_us as u64));
            line.set_value(0)?;
            std::thread::sleep(std::time::Duration::from_micros(time_off_us as u64));
        }
    }
}
