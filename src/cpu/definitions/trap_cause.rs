use crate::cpu::definitions::addresses::{MCAUSE, MEPC, MTVAL, MTVEC, SCAUSE, SEPC, STVAL, STVEC};
use crate::cpu::definitions::cpu::cpu_definition::CPUMode;
use crate::cpu::definitions::{addresses, masks};
use crate::cpu::definitions::masks::{GLOBAL_MIE, GLOBAL_SIE, MCAUSE_INTERRUPT, MPIE, MPP, SPIE, SPP};

// Traps are about transferring control
// To transfer control, you will want to overwrite the pc with an address (source?)
pub const M_TRAP: TrapDestination = TrapDestination {
    epc: MEPC,
    cause: MCAUSE,
    tval: MTVAL,
    tvec: MTVEC,
    pp_mask: MPP,
    ie_mask: GLOBAL_MIE,
    pie_mask: MPIE,
    mode: CPUMode::M
};

pub const S_TRAP: TrapDestination = TrapDestination {
    epc: SEPC,
    cause: SCAUSE,
    tval: STVAL,
    tvec: STVEC,
    pp_mask: SPP,
    ie_mask: GLOBAL_SIE,
    pie_mask: SPIE,
    mode: CPUMode::S
};

pub struct TrapDestination {
    pub epc: usize,
    pub cause: usize,
    pub tval: usize,
    pub tvec: usize,
    pub pp_mask: u32,
    pub ie_mask: u32,
    pub pie_mask: u32,
    pub mode: CPUMode
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
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
    MachineTimerInterrupt,
    InstructionPageFault { address: usize },
    LoadPageFault { address: usize },
    StorePageFault { address: usize },
    MachineExternalInterrupt,
    SupervisorExternalInterrupt,

    SupervisorTimerInterrupt,
}

impl TrapCause {

    pub fn target_mode_for(cause: TrapCause) -> CPUMode {
        // the target mode is ...
        match cause {
            TrapCause::MachineTimerInterrupt => CPUMode::M,
            TrapCause::MachineExternalInterrupt => CPUMode::M,
            TrapCause::SupervisorExternalInterrupt => CPUMode::S,
            TrapCause::SupervisorTimerInterrupt => CPUMode::S,
            _ => panic!("not an interrupt cause")
        }
    }

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
            TrapCause::MachineTimerInterrupt => MCAUSE_INTERRUPT | 7,
            TrapCause::InstructionPageFault { .. } => 12,
            TrapCause::LoadPageFault { .. } => 13,
            TrapCause::StorePageFault { .. } => 15,
            TrapCause::MachineExternalInterrupt => MCAUSE_INTERRUPT | 11,
            TrapCause::SupervisorExternalInterrupt => MCAUSE_INTERRUPT | 9,
            TrapCause::SupervisorTimerInterrupt => MCAUSE_INTERRUPT | 5,
        }
    }
}