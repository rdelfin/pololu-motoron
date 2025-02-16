/// This function encodes a command into a byte vector that can be sent back over the wire to the
/// pololu motoron device.
pub fn encode_command<C: Command>(c: &C) -> Vec<u8> {
    let mut response = Vec::with_capacity(1);
    response.push(c.code());
    response
}

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
    fn encode_body(&self, bytes: &mut [u8]);
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
        fn encode_body(&self, _: &mut [u8]) {}
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
}

pub struct ReadEeprom {
    pub offset: u8,
    pub length: u8,
}
impl Command for ReadEeprom {
    type Response = Vec<u8>;
    plain_code!(0x93);
    plain_byte_count!(2);
}

pub struct WriteEeprom {
    pub offset: u8,
    pub value: u8,
}
impl Command for WriteEeprom {
    type Response = ();
    plain_code!(0x95);
    plain_byte_count!(6);
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
}

pub struct ClearLatchedStatusFlags {
    pub flags: u16,
}
impl Command for ClearLatchedStatusFlags {
    type Response = ();
    plain_code!(0xA9);
    plain_byte_count!(2);
}

pub struct SetLatchedStatusFlags {
    pub flags: u16,
}
impl Command for SetLatchedStatusFlags {
    type Response = ();
    plain_code!(0xAC);
    plain_byte_count!(2);
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
}

pub struct SetAllSpeeds {
    pub mode: SpeedMode,
    pub speed: Vec<i16>,
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
        self.speed.len()
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
}

pub enum MultiDeviceErrorCheckReponse {
    // 0x00
    ErrorActive,
    // 0x3C
    Ok,
}

pub struct MultiDeviceWrite<'a> {
    pub starting_device_number: u8,
    pub device_count: u8,
    // pub bytes_per_device: u8,
    pub command_byte: u8,
    pub payload: &'a [u8],
}
impl<'a> Command for MultiDeviceWrite<'a> {
    type Response = ();
    plain_code!(0xF9);
    fn num_bytes(&self) -> usize {
        4 + self.payload.len()
    }
}
