use crate::commands::{
    decode_response, encode_command, Command, GetFirmwareVersion, SetProtocolOptions,
};
use commands::{SetSpeed, SpeedMode};
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
    #[error("error with command: {0}")]
    Command(#[from] commands::Error),
    #[error("speed provided outside of [-1.0, 1.0] range, value: {0}")]
    InvalidSpeed(f32),
    #[error(
        "provided motor {provided} is higher than the number of supported motors {num_motors}"
    )]
    InvalidMotor { provided: u8, num_motors: u8 },
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

    pub fn set_speed(&mut self, motor_idx: u8, speed: f32) -> Result {
        let num_motors = self.controller_type.motor_channels();
        if speed.abs() > 1. {
            Err(Error::InvalidSpeed(speed))
        } else if motor_idx >= num_motors {
            Err(Error::InvalidMotor {
                provided: motor_idx,
                num_motors,
            })
        } else {
            let speed = (speed * 800.) as i16;
            let cmd = SetSpeed {
                mode: SpeedMode::Normal,
                speed,
                motor: motor_idx + 1,
            };
            self.write_command(&cmd)?;
            Ok(())
        }
    }

    pub fn firmware_version(&mut self) -> Result<FirmwareVersion> {
        let cmd = GetFirmwareVersion;
        self.write_command(&cmd)?;
        let firmware_version = self.read_command(&cmd)?;
        Ok(firmware_version)
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
