use crate::definitions::addresses::{MCAUSE, MEPC, MTVAL, MTVEC, SCAUSE, SEPC, STVAL, STVEC};
use crate::definitions::cpu::cpu_definition::CPUMode;
use crate::definitions::masks;
use crate::definitions::masks::{MCAUSE_INTERRUPT, MPIE};

// Traps are about transferring control
// To transfer control, you will want to overwrite the pc with an address (source?)

pub(crate) struct TrapDestination {
    pub epc: usize,
    pub cause: usize,
    pub tval: usize,
    pub tvec: usize,
    pub pp_mask: u32,
    pub ie_mask: u32,
    pub pie_mask: u32,
    pub mode: CPUMode
}

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
    EnvironmentCallFromMMode,
    EnvironmentCallFromUMode,
    EnvironmentCallFromSMode,
    MachineTimerInterrupt
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
            TrapCause::EnvironmentCallFromSMode => 9,
            TrapCause::EnvironmentCallFromUMode => 8,
            TrapCause::MachineTimerInterrupt => MCAUSE_INTERRUPT | 7
        }
    }
}