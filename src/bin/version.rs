use pololu_motoron::{ControllerType, PololuDevice};

fn main() -> anyhow::Result<()> {
    let mut device = PololuDevice::new(ControllerType::M2T256, "/dev/i2c-1", 0x10)?;
    let version = device.firmware_version()?;
    println!("Firmware version: {version:?}");
    Ok(())
}
