use clap::Parser;
use pololu_motoron::{ClearLatchedStatusFlags, ControllerType};
use std::{path::PathBuf, time::Duration};

/// Program that prints the version of the firmware on a given Pololu Motoron device
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// I2C device we should connect to
    #[arg(short, long, default_value = "/dev/i2c-0")]
    device: PathBuf,

    /// I2C address to address this device with
    #[arg(short, long, default_value_t = 0x10)]
    address: u16,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut device =
        pololu_motoron::Device::new(ControllerType::M2T256, args.device, args.address)?;
    device.reinitialise()?;
    device.clear_latched_status_flags(ClearLatchedStatusFlags {
        reset: true,
        ..Default::default()
    })?;

    loop {
        device.set_all_speeds(&[0.5, 0.8])?;
        std::thread::sleep(Duration::from_secs_f32(0.01));
    }
}
