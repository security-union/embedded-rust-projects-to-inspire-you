use gilrs::{Axis, Button, Event, Gilrs};
use anyhow::Result;

fn main() -> Result<()> {
    let mut gilrs = Gilrs::new().unwrap();
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }
    loop {
        while let Some(Event { id, event, time }) = gilrs.next_event() {
            match event {
                gilrs::EventType::AxisChanged(Axis::LeftStickY, value,code) => {
                    println!("Left Stick Y: {}", value);   
                },
                gilrs::EventType::AxisChanged(Axis::RightStickX, value, code) => {
                    println!("Right Stick X: {}", value);
                },
                _ => {

                }
            }
        }
    }
    Ok(())
}
