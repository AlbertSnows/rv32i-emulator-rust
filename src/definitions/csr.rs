use crate::definitions::trap_cause::TrapCause;
use crate::definitions::cpu_definition::{CPUMode, CPUCycles};
use crate::definitions::addresses::{
    MSTATUS, MTVEC, MEPC, MCAUSE, MTVAL, MCYCLE, MINSTRET, CYCLE, TIME, INSTRET,
};
use crate::utility::bit_operations::mask_and_shift;

const ACCESS_TYPE_LOCATION: u32 = 10;
const MINIMUM_PRIVILEGE_LOCATION: u32 = 8;
const READ_ONLY: u32 = 0b11;

pub fn build_csr_state() -> CSRState {
    CSRState { 
        mstatus: 0,
        mtvec: 0,
        mepc: 0, 
        mcause: 0,
        mtval: 0,
        mcycle: 0,
        minstret: 0,
     }
}

// CSR (Control and Status Register) address space is 12 bits wide (0..4096), per the Zicsr extension 
// separate storage from the general-purpose, not reg  file
#[derive(Debug, Copy, PartialEq, Clone)]
pub struct CSRState {
    mstatus: u32,
    mtvec: u32,
    mepc: u32, 
    mcause: u32,
    mtval: u32,
    mcycle: u32,
    minstret: u32,
}

impl CSRState {

    fn field_for(&mut self, address: usize) -> Result<&mut u32, TrapCause> {
        match address {
            MSTATUS => Ok(&mut self.mstatus),
            MTVEC => Ok(&mut self.mtvec),
            MEPC => Ok(&mut self.mepc),
            MCAUSE => Ok(&mut self.mcause),
            MTVAL => Ok(&mut self.mtval),
            MCYCLE | CYCLE | TIME => Ok(&mut self.mcycle),
            MINSTRET | INSTRET => Ok(&mut self.minstret),
            _ => Err(TrapCause::IllegalInstruction { instruction: None }),
        }
    }

    pub fn read(&self, address: usize) -> Result<u32, TrapCause> {
        match address {
            MSTATUS => Ok(self.mstatus),
            MTVEC => Ok(self.mtvec),
            MEPC => Ok(self.mepc),
            MCAUSE => Ok(self.mcause),
            MTVAL => Ok(self.mtval),
            MCYCLE | CYCLE | TIME => Ok(self.mcycle),
            MINSTRET | INSTRET => Ok(self.minstret),
            _ => Err(TrapCause::IllegalInstruction { instruction: None }),
        }
    }

    pub fn write(&mut self, address: usize, value: u32, current_mode: CPUMode) -> Result<u32, TrapCause> {
        // "The top two bits (csr[11:10]) indicate whether the register is read/write (00, 01, or 10) or read-only (11).
        // The next two bits (csr[9:8]) encode the lowest privilege level that can access the CSR."
        // NOTE: 9:8 means, "In order to write to the CSR, you must have at least this much access"

        // https://docs.riscv.org/reference/isa/_attachments/riscv-privileged.pdf
        // todo: replace 10, 8, and maybe 0b11 with non-magic numbers
        let has_write_access = mask_and_shift(address as u32, 0b11 << ACCESS_TYPE_LOCATION) != READ_ONLY;
        let privilege_level = mask_and_shift(address as u32, 0b11 << MINIMUM_PRIVILEGE_LOCATION);
        let meets_minimum_privilege = privilege_level <= current_mode.as_privilege_level();
        if !has_write_access | !meets_minimum_privilege {
            // todo: encode more info about the specific trap failure?
            return Err(TrapCause::IllegalInstruction { instruction: None });
        }
        let property = self.field_for(address)?;
        *property = value;
        Ok(value)
    }

    pub fn update_cycle(&mut self, cycle: CPUCycles) {
        match cycle {
            CPUCycles::Cycle | CPUCycles::Time => {
                self.mcycle += 1;
            },
            CPUCycles::Instret => {
                self.minstret += 1;
            },
        }
    }
}
