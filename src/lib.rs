use crate::commands::{
    decode_response, encode_command, Command, GetFirmwareVersion, SetProtocolOptions,
};
use i2cdev::core::I2CDevice;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use std::path::Path;

mod commands;
mod controllers;

pub use crate::commands::FirmwareVersion;
pub use crate::controllers::ControllerType;

/// Represents a
pub struct PololuDevice {
    device: LinuxI2CDevice,
    controller_type: ControllerType,
    cmd_crc: bool,
    res_crc: bool,
    i2c_general_call: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I2C error: {0}")]
    I2c(#[from] LinuxI2CError),
    #[error("Error with command: {0}")]
    Command(#[from] commands::Error),
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

impl PololuDevice {
    pub fn new<P: AsRef<Path>>(
        controller_type: ControllerType,
        device: P,
        address: u16,
    ) -> Result<PololuDevice> {
        let mut device = PololuDevice {
            device: LinuxI2CDevice::new(device, address)?,
            controller_type,
            cmd_crc: true,
            res_crc: true,
            i2c_general_call: true,
        };
        device.write_protocol_options()?;
        Ok(device)
    }

    fn write_protocol_options(&mut self) -> Result {
        let cmd = SetProtocolOptions {
            crc_for_commands: self.cmd_crc,
            crc_for_responses: self.res_crc,
            i2c_general_call: self.i2c_general_call,
        };
        self.write_command(&cmd)?;
        Ok(())
    }

    pub fn firmware_version(&mut self) -> Result<FirmwareVersion> {
        let cmd = GetFirmwareVersion;
        self.write_command(&cmd)?;
        let firmware_version = self.read_command(&cmd)?;
        Ok(firmware_version)
    }

    fn write_command<C: Command>(&mut self, cmd: &C) -> Result {
        let data = encode_command(cmd, self.cmd_crc)?;
        self.device.write(&data[..])?;
        Ok(())
    }

    fn read_command<C: Command>(&mut self, cmd: &C) -> Result<C::Response> {
        let response_len = cmd.expected_response_bytes() + if self.res_crc { 1 } else { 0 };
        let mut data = vec![0; response_len];
        self.device.read(&mut data[..])?;
        let response = decode_response::<C>(data, self.res_crc)?;
        Ok(response)
    }
}
