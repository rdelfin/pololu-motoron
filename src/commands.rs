pub trait Command {
    type Response;
    fn code(&self) -> u8;
}

macro_rules! plain_code {
    ($code:literal) => {
        fn code(&self) -> u8 {
            $code
        }
    };
}

pub struct GetFirwmwareVersion;
impl Command for GetFirwmwareVersion {
    type Response = FirmwareVersion;
    plain_code!(0x87);
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
}

pub struct ReadEeprom {
    pub offset: u8,
    pub length: u8,
}
impl Command for ReadEeprom {
    type Response = Vec<u8>;
    plain_code!(0x93);
}

pub struct WriteEeprom {
    pub offset: u8,
    pub value: u8,
}
impl Command for WriteEeprom {
    type Response = ();
    plain_code!(0x95);
}

pub struct GetVariables {
    pub motor: u8,
    pub offset: u8,
    pub length: u8,
}
impl Command for GetVariables {
    type Response = Vec<u8>;
    plain_code!(0x9A);
}

pub struct SetVariable {
    pub motor: u8,
    pub offset: u8,
    pub value: u16,
}
impl Command for SetVariable {
    type Response = ();
    plain_code!(0x9C);
}

pub struct ClearMotorFault {
    pub unconditional: bool,
}
impl Command for ClearMotorFault {
    type Response = ();
    plain_code!(0xA6);
}

pub struct ClearLatchedStatusFlags {
    pub flags: u16,
}
impl Command for ClearLatchedStatusFlags {
    type Response = ();
    plain_code!(0xA9);
}

pub struct SetLatchedStatusFlags {
    pub flags: u16,
}
impl Command for SetLatchedStatusFlags {
    type Response = ();
    plain_code!(0xAC);
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
}

pub struct SetAllSpeeds {
    pub mode: SpeedMode,
    pub speed: i16,
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
}

pub struct MultiDeviceErrorCheck {
    pub starting_device_number: u8,
    pub device_count: u8,
}
impl Command for MultiDeviceErrorCheck {
    type Response = MultiDeviceErrorCheckReponse;
    plain_code!(0xF5);
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
}
