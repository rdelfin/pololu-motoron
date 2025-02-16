use std::ops::Range;

/// This function encodes a command into a byte vector that can be sent back over the wire to the
/// pololu motoron device.
pub fn encode_command<C: Command>(c: &C, with_crc: bool) -> Result<Vec<u8>> {
    let len = 1 + c.num_bytes() + if with_crc { 1 } else { 0 };
    let mut response = vec![0; len];
    response[0] = c.code();
    c.encode_body(&mut response[1..])?;
    if with_crc {
        response[len - 1] = get_crc(&response[..len - 1]);
    }
    Ok(response)
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("field {field} has an invalid value (is {value}, can only be between {min} and {max}")]
    InvalidValue {
        min: i32,
        max: i32,
        value: i32,
        field: &'static str,
    },
    #[error("creating multi-device write with commands of different length, all should be the same length")]
    DifferentLengthMultiDeviceWrite,
    #[error(
        "creating multi-device write with different command codes, all should be the same code"
    )]
    DifferentCodeMultiDeviceWrite,
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Any type implementing this trait represents a unique command that can be sent over i2c to a
/// pololu motoron controller. Each command will provide an easy-to-use interface to provide the
/// data and this trait together with [`encode_command`] will provide a way of interacting with the
/// protocol.
pub trait Command {
    type Response;
    /// This is the command code of this command used in the i2c protocol
    fn code(&self) -> u8;
    /// This is the number of bytes used up by the arguments **EXCLUDING THE COMMAND CODE** over
    /// the wire
    fn num_bytes(&self) -> usize;
    /// This function will write the corresponding body of the command into the `bytes` argument.
    /// Note that we expect that `bytes` be at least as long as the length returned by the
    /// [`Self::num_bytes`] function. If it's longer, we will only write out the number of bytes
    /// returned by `num_bytes`.
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()>;
}

macro_rules! plain_code {
    ($code:literal) => {
        fn code(&self) -> u8 {
            $code
        }
    };
}

macro_rules! plain_byte_count {
    ($bytes:literal) => {
        fn num_bytes(&self) -> usize {
            $bytes
        }
    };
}

macro_rules! noop_encode {
    () => {
        fn encode_body(&self, _: &mut [u8]) -> Result<()> {
            Ok(())
        }
    };
}

macro_rules! check_value {
    ($self:ident, $field:ident, $min:literal, $max:literal $(,)?) => {
        #[allow(unused_comparisons)]
        if $self.$field < $min || $self.$field > $max {
            return Err(Error::InvalidValue {
                min: $min,
                max: $max,
                value: $self.$field.into(),
                field: stringify!($field),
            });
        }
    };
}
macro_rules! check_value_expr {
    ($expr:ident, $min:literal, $max:literal, $field_name:literal $(,)?) => {
        if $expr < $min || $expr > $max {
            return Err(Error::InvalidValue {
                min: $min,
                max: $max,
                value: $expr.into(),
                field: $field_name,
            });
        }
    };
}

pub struct GetFirwmwareVersion;
impl Command for GetFirwmwareVersion {
    type Response = FirmwareVersion;
    plain_code!(0x87);
    plain_byte_count!(0);
    noop_encode!();
}

pub struct FirmwareVersion {
    product_id: u16,
    minor_fw_version: u8,
    major_fw_version: u8,
}

pub struct SetProtocolOptions {
    pub crc_for_commands: bool,
    pub crc_for_responses: bool,
    pub i2c_general_call: bool,
}
impl Command for SetProtocolOptions {
    type Response = ();
    plain_code!(0x8B);
    plain_byte_count!(2);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        let options_byte = u8::from(self.crc_for_commands)
            | (u8::from(self.crc_for_responses) << 1)
            | (u8::from(self.i2c_general_call) << 2);
        bytes[0] = options_byte;
        write_inverted_bytes(bytes, 0..1, 1);
        Ok(())
    }
}

pub struct ReadEeprom {
    pub offset: u8,
    pub length: u8,
}
impl Command for ReadEeprom {
    type Response = Vec<u8>;
    plain_code!(0x93);
    plain_byte_count!(2);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, offset, 0, 0x7F);
        check_value!(self, length, 1, 32);
        bytes[0] = self.offset;
        bytes[1] = self.length;
        Ok(())
    }
}

pub struct WriteEeprom {
    pub offset: u8,
    pub value: u8,
}
impl Command for WriteEeprom {
    type Response = ();
    plain_code!(0x95);
    plain_byte_count!(6);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, offset, 0, 0x7F);
        bytes[0] = self.offset;
        bytes[1] = u8::from((self.value & 0x80) != 0);
        bytes[2] = self.value & 0x7F;
        write_inverted_bytes(bytes, 0..3, 3);
        Ok(())
    }
}

pub struct Reinitialise;
impl Command for Reinitialise {
    type Response = ();
    plain_code!(0x96);
    plain_byte_count!(0);
    noop_encode!();
}

pub struct Reset;
impl Command for Reset {
    type Response = ();
    plain_code!(0x99);
    plain_byte_count!(0);
    noop_encode!();
}

pub struct GetVariables {
    pub motor: u8,
    pub offset: u8,
    pub length: u8,
}
impl Command for GetVariables {
    type Response = Vec<u8>;
    plain_code!(0x9A);
    plain_byte_count!(3);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, motor, 0, 3);
        check_value!(self, offset, 0, 0x7F);
        check_value!(self, length, 1, 32);
        bytes[0] = self.motor;
        bytes[1] = self.offset;
        bytes[2] = self.length;
        Ok(())
    }
}

pub struct SetVariable {
    pub motor: u8,
    pub offset: u8,
    pub value: u16,
}
impl Command for SetVariable {
    type Response = ();
    plain_code!(0x9C);
    plain_byte_count!(4);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, motor, 0, 3);
        check_value!(self, offset, 0, 0x7F);
        check_value!(self, value, 0, 0x3FFF);
        bytes[0] = self.motor;
        bytes[1] = self.offset;
        bytes[2] = (self.value & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        bytes[3] = ((self.value >> 7) & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        Ok(())
    }
}

pub struct CoastNow;
impl Command for CoastNow {
    type Response = ();
    plain_code!(0xA5);
    plain_byte_count!(0);
    noop_encode!();
}

pub struct ClearMotorFault {
    pub unconditional: bool,
}
impl Command for ClearMotorFault {
    type Response = ();
    plain_code!(0xA6);
    plain_byte_count!(1);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        bytes[0] = self.unconditional.into();
        Ok(())
    }
}

pub struct ClearLatchedStatusFlags {
    pub flags: u16,
}
impl Command for ClearLatchedStatusFlags {
    type Response = ();
    plain_code!(0xA9);
    plain_byte_count!(2);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, flags, 0, 0x3FF);
        bytes[0] = (self.flags & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        bytes[1] = ((self.flags >> 7) & 0x7)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        Ok(())
    }
}

pub struct SetLatchedStatusFlags {
    pub flags: u16,
}
impl Command for SetLatchedStatusFlags {
    type Response = ();
    plain_code!(0xAC);
    plain_byte_count!(2);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, flags, 0, 0x3FF);
        bytes[0] = (self.flags & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        bytes[1] = ((self.flags >> 7) & 0x7)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        Ok(())
    }
}

pub enum SpeedMode {
    Normal,
    Now,
    Buffered,
}

pub struct SetSpeed {
    pub mode: SpeedMode,
    pub motor: u8,
    pub speed: i16,
}
impl Command for SetSpeed {
    type Response = ();
    fn code(&self) -> u8 {
        match self.mode {
            SpeedMode::Normal => 0xD1,
            SpeedMode::Now => 0xD2,
            SpeedMode::Buffered => 0xD4,
        }
    }
    plain_byte_count!(3);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, motor, 0, 3);
        check_value!(self, speed, -800, 800);
        // SAFETY: we assume this system uses a 2's compliment representation of signed integers.
        // Regardless, an i16 can be safely interpreted as a u16 as all possible 16-bit
        // representations are valid in both.
        let speed_as_2c: u16 = unsafe { std::mem::transmute(self.speed) };
        bytes[0] = self.motor;
        bytes[1] = (speed_as_2c & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        bytes[2] = ((speed_as_2c >> 7) & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        Ok(())
    }
}

pub struct SetAllSpeeds {
    pub mode: SpeedMode,
    pub speeds: Vec<i16>,
}
impl Command for SetAllSpeeds {
    type Response = ();
    fn code(&self) -> u8 {
        match self.mode {
            SpeedMode::Normal => 0xE1,
            SpeedMode::Now => 0xE2,
            SpeedMode::Buffered => 0xE4,
        }
    }
    fn num_bytes(&self) -> usize {
        self.speeds.len() * 2
    }
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        for (idx, speed) in self.speeds.iter().enumerate() {
            let speed = *speed;
            check_value_expr!(speed, -800, 800, "speeds");
            // SAFETY: we assume this CPU uses a 2's compliment representation of signed integers.
            // Regardless, an i16 can be safely interpreted as a u16 as all possible 16-bit
            // representations are valid in both.
            let speed_as_2c: u16 = unsafe { std::mem::transmute(speed) };
            bytes[idx * 2] = (speed_as_2c & 0x7F)
                .try_into()
                .expect("could not convert u16 to u8 with mask");
            bytes[idx * 2 + 1] = ((speed_as_2c >> 7) & 0x7F)
                .try_into()
                .expect("could not convert u16 to u8 with mask");
        }
        Ok(())
    }
}

pub enum SpeedModeNoBuffer {
    Normal,
    Now,
}

pub struct SetAllSpeedsUsingBuffers {
    pub mode: SpeedModeNoBuffer,
}
impl Command for SetAllSpeedsUsingBuffers {
    type Response = ();
    fn code(&self) -> u8 {
        match self.mode {
            SpeedModeNoBuffer::Normal => 0xF0,
            SpeedModeNoBuffer::Now => 0xF3,
        }
    }
    plain_byte_count!(0);
    noop_encode!();
}

pub enum BrakingMode {
    Normal,
    Now,
}

pub struct SetBraking {
    pub mode: BrakingMode,
    pub motor: u8,
    pub ammount: u16,
}
impl Command for SetBraking {
    type Response = ();
    fn code(&self) -> u8 {
        match self.mode {
            BrakingMode::Normal => 0xB1,
            BrakingMode::Now => 0xB2,
        }
    }
    plain_byte_count!(3);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, motor, 1, 3);
        check_value!(self, ammount, 0, 800);
        bytes[0] = self.motor;
        bytes[1] = (self.ammount & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        bytes[2] = ((self.ammount >> 7) & 0x7F)
            .try_into()
            .expect("could not convert u16 to u8 with mask");
        Ok(())
    }
}

pub struct ResetCommandTimeout;
impl Command for ResetCommandTimeout {
    type Response = ();
    plain_code!(0xF5);
    plain_byte_count!(0);
    noop_encode!();
}

pub struct MultiDeviceErrorCheck {
    pub starting_device_number: u8,
    pub device_count: u8,
}
impl Command for MultiDeviceErrorCheck {
    type Response = MultiDeviceErrorCheckReponse;
    plain_code!(0xF5);
    plain_byte_count!(2);
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, starting_device_number, 0, 0x7F);
        check_value!(self, device_count, 0, 0x7F);
        bytes[0] = self.starting_device_number;
        bytes[1] = self.device_count;
        Ok(())
    }
}

pub enum MultiDeviceErrorCheckReponse {
    // 0x00
    ErrorActive,
    // 0x3C
    Ok,
}

pub struct MultiDeviceWrite<C: Command> {
    pub starting_device_number: u8,
    pub device_count: u8,
    pub commands: Vec<C>,
}
impl<C: Command> Command for MultiDeviceWrite<C> {
    type Response = ();
    plain_code!(0xF9);
    fn num_bytes(&self) -> usize {
        let commands_bytes: usize = self.commands.iter().map(|c| c.num_bytes()).sum();
        4 + commands_bytes
    }
    fn encode_body(&self, bytes: &mut [u8]) -> Result<()> {
        check_value!(self, starting_device_number, 0, 0x7F);
        check_value!(self, device_count, 0, 0x7F);
        if self.commands.len() > 0x7F {
            return Err(Error::InvalidValue {
                min: 0,
                max: 0x7F,
                value: self.commands.len().try_into().unwrap(),
                field: "commands",
            });
        }
        let command_length = self.commands.first().map(|c| c.num_bytes()).unwrap_or(0);
        let code = self.commands.first().map(|c| c.code()).unwrap_or(0);

        bytes[0] = self.starting_device_number;
        bytes[1] = self.device_count;
        bytes[2] = command_length
            .try_into()
            .expect("command length guaranteed to be under 0x7F");
        bytes[3] = code;
        let mut start_idx = 4;
        for cmd in &self.commands {
            if cmd.code() != code {
                return Err(Error::DifferentCodeMultiDeviceWrite);
            }
            if cmd.num_bytes() != command_length {
                return Err(Error::DifferentLengthMultiDeviceWrite);
            }
            cmd.encode_body(&mut bytes[start_idx..])?;
            start_idx += command_length;
        }
        Ok(())
    }
}

fn get_crc(message: &[u8]) -> u8 {
    let mut crc = 0;
    // for (uint8_t i = 0; i < length; i++)
    for byte in message {
        crc ^= byte;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc ^= 0x91;
            }
            crc >>= 1;
        }
    }
    crc
}

fn write_inverted_bytes(data: &mut [u8], orig: Range<usize>, write_offset: usize) {
    if write_offset + orig.len() > data.len() {
        panic!("not enough bytes in data to do an invert of the length required.");
    }

    for i in orig {
        data[i + write_offset] = data[i] ^ 0x7F;
    }
}
