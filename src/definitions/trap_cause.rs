// Traps are about transferring control
// To transfer control, you will want to overwirte the pc with an address (source?)

#[derive(Debug, PartialEq)]
pub enum TrapCause {
    InstructionAddressMisaligned  { address: usize }, 
    InstructionAccessFault  { address: usize }, 
    LoadAddressMisaligned { address: usize }, 
    LoadAccessFault { address: usize }, 
    StoreAddressMisaligned { address: usize }, 
    StoreAccessFault { address: usize }, 
    IllegalInstruction { instruction: Option<u32> }, 
    Breakpoint, 
    EnvironmentCallFromMMode 
}

impl TrapCause {
    pub fn mcause_code(&self) -> u32 {
        match self {
            TrapCause::InstructionAddressMisaligned { .. } => 0,
            TrapCause::InstructionAccessFault { .. } => 1,
            TrapCause::IllegalInstruction { .. } => 2,
            TrapCause::Breakpoint => 3,
            TrapCause::LoadAddressMisaligned { .. } => 4,
            TrapCause::LoadAccessFault { .. } => 5,
            TrapCause::StoreAddressMisaligned { .. } => 6,
            TrapCause::StoreAccessFault { .. } => 7,
            TrapCause::EnvironmentCallFromMMode => 11,
        }
    }
}