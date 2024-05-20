use gpio_cdev::{Chip, LineRequestFlags};
use std::thread::sleep;
use std::time::Duration;

const GPIO5: u32 = 5;
const GPIO6: u32 = 6;

const PWM_FREQUENCY_HZ: f64 = 200.0;
const DUTY_CYCLE: f64 = 60.0;

enum Direction {
    Left,
    Right,
}

/**
 * This program will turn a motor left and right every 5 seconds.
 */
fn main() -> Result<(), gpio_cdev::Error> {
    // 1. Open the GPIO chip; it's gpiochip0 for the main GPIO controller on a Raspberry Pi
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let line5 = chip.get_line(GPIO5)?;
    let line6 = chip.get_line(GPIO6)?;
    // Initially both are off
    let line5 = line5.request(LineRequestFlags::OUTPUT, 0, "pwm")?;
    let line6 = line6.request(LineRequestFlags::OUTPUT, 0, "pwm")?;

    // 2. Initially the motor is turning left
    let mut current_direction = Direction::Left;

    // 3. Keep track of the last time we changed direction
    let mut last_direction_change = std::time::Instant::now();

    let time_on_ms = (DUTY_CYCLE / 100.0) * (1.0 / PWM_FREQUENCY_HZ) * 1000.0;
    let time_off_ms = ((100.0 - DUTY_CYCLE) / 100.0) * (1.0 / PWM_FREQUENCY_HZ) * 1000.0;
    println!("Time on: {} ms, Time off: {} ms", time_on_ms, time_off_ms);

    loop {
        // transform direction to GPIO signals
        match current_direction {
            Direction::Left => {
                line5.set_value(1)?;
                line6.set_value(0)?;
            }
            Direction::Right => {
                line5.set_value(0)?;
                line6.set_value(1)?;
            }
        }

        sleep(Duration::from_millis(time_on_ms as u64));
        // turn it off
        line5.set_value(0)?;
        line6.set_value(0)?;
        sleep(Duration::from_millis(time_off_ms as u64));
        
        if last_direction_change.elapsed().as_secs() >= 5 {     
            // Change direction
            current_direction = match current_direction {
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left
            };
            last_direction_change = std::time::Instant::now();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
