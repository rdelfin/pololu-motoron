//! This is a Rust driver for the
//! [Pololu Motoron motor controller](https://www.pololu.com/docs/0J84) written to work on Linux.
//! This provides an easy-to-use interface to control motors, configure the controller, and get
//! information out of it while maintaining flexibility. For example, if you wanted to talk to a
//! motor:
//!
//! ```no_run
//! use pololu_motoron::ControllerType;
//! use std::time::Duration;
//!
//! # fn main() -> anyhow::Result<()> {
//! let mut device = pololu_motoron::Device::new(ControllerType::M2T256, "/dev/i2c-0", 0x10)?;
//!
//! // Get version information
//! let version = device.firmware_version();
//! println!("Version: {version:?}");
//!
//! // Move wheels in opposite directions indefinitely
//! loop {
//!     device.set_speed(0, 1.0);
//!     device.set_speed(1, -1.0);
//!     std::thread::sleep(Duration::from_millis(5));
//! }
//! # }
//! ```
//!
//! We recommend starting with the [`Device`] documentation.

use crate::commands::{
    decode_response, encode_command, Command, GetFirmwareVersion, SetProtocolOptions,
};
use commands::{
    Reinitialise, SetAllSpeeds, SetAllSpeedsUsingBuffers, SetSpeed, SpeedMode, SpeedModeNoBuffer,
};
use i2cdev::core::I2CDevice;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};
use std::path::Path;
use std::time::Duration;

mod commands;
mod controllers;

pub use crate::commands::{ClearLatchedStatusFlags, Error as CommandsError, FirmwareVersion};
pub use crate::controllers::ControllerType;

/// Represents a Pololu Motoron motor controller. Use this to control a single motor controller on
/// a given bus.
pub struct Device {
    device: LinuxI2CDevice,
    controller_type: ControllerType,
    cmd_crc: bool,
    res_crc: bool,
    i2c_general_call: bool,
}

/// The generic error returned by all functions in this module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Any errors returned by the I2C bus/device itself. Includes permission errors, resource busy
    /// errors, among others
    #[error("I2C error: {0}")]
    I2c(#[from] LinuxI2CError),

    /// Any errors related to the command itself. Please refer to [`CommandsError`] for more
    /// details.
    #[error("error with command: {0}")]
    Command(#[from] CommandsError),

    /// Returned when the speed provided to one of the motor speed functions is out of range. We
    /// expect speed to be in the range `[-1.0, 1.0]`, so if it's not this error is returned with
    /// the incorrect speed included.
    #[error("speed provided outside of [-1.0, 1.0] range, value: {0}")]
    InvalidSpeed(f32),

    /// Returned when the user requests an invalid motor ID. This happens when you provide an index
    /// higher than or equal to the number of motors (zero-based index)
    #[error(
        "provided motor {provided} is higher than the number of supported motors {num_motors}"
    )]
    InvalidMotor { provided: u8, num_motors: u8 },

    /// Returned when setting all speeds, if you don't provide the correct number of speeds. How
    /// many speeds have to be provided depends on the controller type, but can be anywhere from 1
    /// to 3.
    #[error(
        "in setting all speeds, you provided {provided} speeds, but this controller has {actual} motors"
    )]
    IncorrectNumberSpeeds { provided: u8, actual: u8 },
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

impl Device {
    /// Create a new device object.
    ///
    /// # Arguments
    /// * `controller_type` - The type of motor controller being commanded. While the protocol
    ///                       between different Pololu Motoron controllers is the same, this
    ///                       provides us with limits and features of yous specific controller,
    ///                       such as the number of motors available.
    /// * `device`          - Represents the device file of the I2C bus. Usually something like
    ///                       `/dev/i2c-0`.
    /// * `address`         - The I2C address of the device we're talking to. If unconfigured, it
    ///                       will be 0x10 (aka 16).
    pub fn new<P: AsRef<Path>>(
        controller_type: ControllerType,
        device: P,
        address: u16,
    ) -> Result<Device> {
        let mut device = Device {
            device: LinuxI2CDevice::new(device, address)?,
            controller_type,
            cmd_crc: true,
            res_crc: true,
            i2c_general_call: true,
        };
        device.write_protocol_options()?;
        Ok(device)
    }

    /// Reinitialises the device and returns all variables back to default values (though we do
    /// re-write the protocol options before returning).
    pub fn reinitialise(&mut self) -> Result {
        self.write_command(&Reinitialise)?;
        self.write_protocol_options()
    }

    /// This disables all CRC checks on the device, both command and resposnse checks
    pub fn disable_crc(&mut self) -> Result {
        self.cmd_crc = false;
        self.res_crc = false;
        self.write_protocol_options()
    }

    /// This enables all CRC checks on the device, both command and resposnse checks
    pub fn enable_crc(&mut self) -> Result {
        self.cmd_crc = true;
        self.res_crc = true;
        self.write_protocol_options()
    }

    /// Resets the device fully, similar to a power reboot.We also re-write the protocol options
    /// before returning).
    pub fn reset(&mut self) -> Result {
        self.write_command(&Reinitialise)?;
        std::thread::sleep(Duration::from_millis(10));
        self.write_protocol_options()
    }

    /// Call this function to set the speed of a specific motor. Note that speeds reset back to 0
    /// if new commands are not sent in a long time, so expect to send this on a loop if you want
    /// to keep movement.
    ///
    /// # Arguments
    /// * `motor_idx` - The index of the motor, zero-indexed. The most motors supported by one of
    ///                 these devices is 3, so it should be no higher than 2.
    /// * `speed`     - The speed to set the motor to, as a floating point between -1.0 and 1.0.
    pub fn set_speed(&mut self, motor_idx: u8, speed: f32) -> Result {
        let cmd = self.get_speed_cmd(motor_idx, speed, SpeedMode::Normal)?;
        self.write_command(&cmd)
    }

    /// Call this function to set the speed of all motors simultaneously. Note that, much like
    /// [`Device::set_speed`], speeds reset back to 0 if new commands are not sent in a long time,
    /// so expect to send this on a loop if you want to keep movement.
    ///
    /// # Arguments
    /// * `speeds` - The speeds to set the motors to, as floating points between -1.0 and 1.0. Note
    ///              that the length of the array MUST match the number of supported motor channels
    ///              for your controller type. If you're not sure how many that is, you can call
    ///              the [`ControllerType::motor_channels`] function.
    pub fn set_all_speeds(&mut self, speeds: &[f32]) -> Result {
        let num_motors = self.controller_type.motor_channels();
        if usize::from(num_motors) != speeds.len() {
            return Err(Error::IncorrectNumberSpeeds {
                provided: speeds.len().try_into().unwrap(),
                actual: num_motors,
            });
        }
        let speeds = speeds
            .into_iter()
            .map(|speed| {
                if speed.abs() > 1. {
                    Err(Error::InvalidSpeed(*speed))
                } else {
                    Ok((*speed * 800.) as i16)
                }
            })
            .collect::<Result<_>>()?;
        let cmd = SetAllSpeeds {
            mode: SpeedMode::Normal,
            speeds,
        };
        self.write_command(&cmd)
    }

    /// Call this function to set the speed of multiple motors simultaneously. Note that, much like
    /// [`Device::set_speed`], speeds reset back to 0 if new commands are not sent in a long time,
    /// so expect to send this on a loop if you want to keep movement.
    ///
    /// # Arguments
    /// * `speeds` - A list of pairs of motor indeces and speeds to set the motors to, as floating
    ///              points between -1.0 and 1.0. Note that the indeces must be between 0 and the
    ///              max number of motors on your specific controller. If you provide the same
    ///              index more than once, we will simply send an additional command that will
    ///              override the first, but we recommend against it as it wastes bandwidth and
    ///              time on the i2c bus.
    pub fn set_multi_speed(&mut self, speeds: &[(u8, f32)]) -> Result {
        // First buffer all the requested speeds
        let cmds = speeds
            .into_iter()
            .map(|(motor_idx, speed)| self.get_speed_cmd(*motor_idx, *speed, SpeedMode::Buffered))
            .collect::<Result<Vec<_>>>()?;
        for cmd in cmds {
            self.write_command(&cmd)?;
        }

        // Then commit them to the controller for simultaneous action
        let cmd = SetAllSpeedsUsingBuffers {
            mode: SpeedModeNoBuffer::Normal,
        };
        self.write_command(&cmd)
    }

    pub fn clear_latched_status_flags(&mut self, flags: ClearLatchedStatusFlags) -> Result {
        self.write_command(&flags)
    }

    /// Call this function to obtain the firmware version reported by the device.
    pub fn firmware_version(&mut self) -> Result<FirmwareVersion> {
        let cmd = GetFirmwareVersion;
        self.write_command(&cmd)?;
        let firmware_version = self.read_command(&cmd)?;
        Ok(firmware_version)
    }

    fn get_speed_cmd(&self, motor_idx: u8, speed: f32, mode: SpeedMode) -> Result<SetSpeed> {
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
            Ok(SetSpeed {
                mode,
                speed,
                motor: motor_idx + 1,
            })
        }
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
        println!("Writing command: {data:?}");
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
