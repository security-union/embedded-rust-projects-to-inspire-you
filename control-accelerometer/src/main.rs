use byteorder::{ByteOrder, LittleEndian};
use gpio_cdev::{Chip, LineHandle, LineRequestFlags};
use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};
use std::io::Write;
use std::thread;
use std::time::Duration;

const ACC_CONVERSION: f32 = 2.0 * 16.0 / 8192.0;
const REG_READ: u8 = 0x80;
const REG_MULTI_BYTE: u8 = 0x40;
const REG_BW_RATE: u8 = 0x2C;
const REG_POWER_CTL: u8 = 0x2D;
const REG_DATA_START: u8 = 0x32;
const REG_DATA_FORMAT: u8 = 0x31;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup SPI
    let mut spi = Spidev::open("/dev/spidev0.0")?;
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(2_000_000) // 2 MHz
        .mode(SpiModeFlags::SPI_MODE_3)
        .build();
    spi.configure(&options)?;

    // Setup GPIO for CS pins using gpio_cdev
    let mut chip = Chip::new("/dev/gpiochip0")?;
    let cs_pins = vec![chip.get_line(6)?, chip.get_line(17)?, chip.get_line(22)?];
    let mut cs_handles: Vec<LineHandle> = cs_pins
        .into_iter()
        .map(|line| line.request(LineRequestFlags::OUTPUT, 1, "spi-cs").unwrap())
        .collect();

    // Initialize ADXL345
    for cs_handle in &mut cs_handles {
        init_adxl345(&mut spi, cs_handle)?;
        thread::sleep(Duration::from_secs(1)); // Short delay after initialization
        let device_id = read_register(&mut spi, cs_handle, 0x00, 1)?;
        println!(
            "ADXL345 on CS pin {:?} has device ID: {:?}",
            cs_handle, device_id
        );
        if device_id[0] != 0xE5 {
            println!("ADXL345 on CS is not communicating properly");
            std::process::exit(1);
        }
    }

    // Main loop
    loop {
        for cs_handle in &mut cs_handles {
            thread::sleep(Duration::from_millis(10));
            let (x, y, z) = read_acceleration(&mut spi, cs_handle)?;
            let pin = cs_handle.line().offset();
            println!("ADXL345 pin={} x={:.3}, y={:.3}, z={:.3}", pin, x, y, z);
        }
    }
}

fn write_register(
    spi: &mut Spidev,
    cs: &mut LineHandle,
    reg_address: u8,
    data: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    cs.set_value(0)?; // Set CS low
    spi.write(&[reg_address, data])?;
    cs.set_value(1)?; // Set CS high
    Ok(())
}

fn read_register(
    spi: &mut Spidev,
    cs: &mut LineHandle,
    reg_address: u8,
    length: usize,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    cs.set_value(0)?; // Set CS low
    let command = vec![reg_address | REG_READ | (if length > 1 { REG_MULTI_BYTE } else { 0 })];
    let tx_buf = command
        .iter()
        .cloned()
        .chain(std::iter::repeat(0).take(length))
        .collect::<Vec<_>>();
    let mut rx_buf = vec![0u8; length + 1]; // +1 for the dummy byte
    {
        let mut transfer = SpidevTransfer::read_write(&tx_buf, &mut rx_buf);
        spi.transfer(&mut transfer)?;
    }
    cs.set_value(1)?;

    Ok(rx_buf[1..].to_vec()) // Ignore the first byte
}

fn init_adxl345(spi: &mut Spidev, cs: &mut LineHandle) -> Result<(), Box<dyn std::error::Error>> {
    write_register(spi, cs, REG_BW_RATE, 0x0F)?; // 100 Hz
    write_register(spi, cs, REG_DATA_FORMAT, 0x0B)?; // +/- 4g; 0.004g/LSB
    write_register(spi, cs, REG_POWER_CTL, 0x08)?; // Measurement mode
    Ok(())
}

fn read_acceleration(
    spi: &mut Spidev,
    cs: &mut LineHandle,
) -> Result<(f32, f32, f32), Box<dyn std::error::Error>> {
    let data = read_register(spi, cs, REG_DATA_START, 6)?;
    let x = LittleEndian::read_i16(&data[0..2]) as f32 * ACC_CONVERSION;
    let y = LittleEndian::read_i16(&data[2..4]) as f32 * ACC_CONVERSION;
    let z = LittleEndian::read_i16(&data[4..6]) as f32 * ACC_CONVERSION;
    Ok((x, y, z))
}
