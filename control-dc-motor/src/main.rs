use gpio_cdev::{Chip, LineRequestFlags};

const GPIO17: u32 = 17;
const GPIO18: u32 = 18;

// PWM Min duty cycle in ms
const PWM_FREQUENCY_HZ: f64 = 100.0;

enum Direction {
    Left,
    Right,
}

fn main() -> Result<(), gpio_cdev::Error> {
    // Open the GPIO chip; usually, it's gpiochip0 for the main GPIO controller on a Raspberry Pi
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let mut current_direction = Direction::Left;
    let mut last_direction_change = std::time::Instant::now();
    let line17 = chip.get_line(GPIO17)?;
    let line18 = chip.get_line(GPIO18)?;
    
    // Initially both are off
    let line17 = line17.request(LineRequestFlags::OUTPUT, 0, "pwm")?;
    let line18 = line18.request(LineRequestFlags::OUTPUT, 0, "pwm")?;

    let duty_cycle = 99.0;
    let time_on_ms = (duty_cycle / 100.0) * (1.0 / PWM_FREQUENCY_HZ) * 1000.0;
    let time_off_ms = ((100.0 - duty_cycle) / 100.0) * (1.0 / PWM_FREQUENCY_HZ) * 1000.0;

    println!("Time on: {} ms, Time off: {} ms", time_on_ms, time_off_ms);

    loop {
        // Set the PWM frequency
        // turn it on
        match current_direction {
            Direction::Left => {
                line17.set_value(1)?;
                line18.set_value(0)?;
            }
            Direction::Right => {
                line17.set_value(0)?;
                line18.set_value(1)?;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(time_on_ms as u64));
        // turn it off
        line17.set_value(0)?;
        line18.set_value(0)?;
        std::thread::sleep(std::time::Duration::from_millis(time_off_ms as u64));
        // change direction
        if last_direction_change.elapsed().as_secs() >= 5 {
            match current_direction {
                Direction::Left => {
                    current_direction = Direction::Right;
                }
                Direction::Right => {
                    current_direction = Direction::Left;
                }
            }
            last_direction_change = std::time::Instant::now();
            // let the motor stop for a while
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    Ok(())
}
