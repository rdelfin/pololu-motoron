/// Reprents the controller type being worked on. If you're not sure which one you have or what
/// capabilities it has, you can consult [this document](https://www.pololu.com/docs/0J84/1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerType {
    M1T550,
    M1U550,
    M2T550,
    M2U550,
    M1T256,
    M1U256,
    M2T256,
    M2U256,
    M3S550,
    M3H550,
    M3S256,
    M3H256,
    M2S24v14,
    M2H24v14,
    M2S24v16,
    M2H24v16,
    M2S18v18,
    M2H18v18,
    M2S18v20,
    M2H18v20,
}

impl ControllerType {
    /// This function returns how many motor channels this specific controller supports, usually
    /// from 1 to 3.
    pub fn motor_channels(&self) -> u8 {
        match self {
            ControllerType::M1T550
            | ControllerType::M1U550
            | ControllerType::M1T256
            | ControllerType::M1U256 => 1,
            ControllerType::M2T550
            | ControllerType::M2U550
            | ControllerType::M2T256
            | ControllerType::M2U256
            | ControllerType::M2S24v14
            | ControllerType::M2H24v14
            | ControllerType::M2S24v16
            | ControllerType::M2H24v16
            | ControllerType::M2S18v18
            | ControllerType::M2H18v18
            | ControllerType::M2S18v20
            | ControllerType::M2H18v20 => 2,
            ControllerType::M3S550
            | ControllerType::M3H550
            | ControllerType::M3S256
            | ControllerType::M3H256 => 3,
        }
    }
}
