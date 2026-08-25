use crate::definitions::trap_cause::TrapCause;
use crate::definitions::cpu::cpu_definition::CPUMode;
use crate::definitions::addresses;
use crate::utility::bit_operations::{mask_and_shift, set_bit_range};
use crate::definitions::masks;

const ACCESS_TYPE_LOCATION: u32 = 10;
const MINIMUM_PRIVILEGE_LOCATION: u32 = 8;
const READ_ONLY: u32 = 0b11;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CPUCycles {
    Cycle,
    Instret,
    Time
}

pub fn build_csr_state() -> CSRState {
    CSRState { 
        mstatus: 0,
        mtvec: 0,
        mepc: 0, 
        mcause: 0,
        mtval: 0,
        mcycle: 0,
        minstret: 0,
        mie: 0,
        mip: 0,
        //
        stvec: 0,
        sscratch: 0,
        sepc: 0,
        scause: 0,
        stval: 0,
        medeleg: 0,
        mideleg: 0,
     }
}

// CSR (Control and Status Register) address space is 12 bits wide (0..4096), per the Zicsr extension 
// separate storage from the general-purpose, not reg file
#[derive(Debug, Copy, PartialEq, Clone)]
pub struct CSRState {
    // todo: look into bit flags, bit field, crate
    mstatus: u32,
    mtvec: u32,
    mepc: u32, 
    mcause: u32,
    mtval: u32,
    mcycle: u32,
    minstret: u32,
    mie: u32,
    mip: u32,
    // hart: hardware thread; a term for one independent instruction execution unit, aka a core
    // mhartid: u32 // machine hart id
    stvec: u32,
    sscratch: u32,
    sepc: u32,
    scause: u32,
    stval: u32,
    medeleg: u32,
    mideleg: u32,

}

// Which interrupt-pending bit in mip is being updated. Only MTI exists for
// now since it's the only interrupt source currently implemented
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MIPBits {
    MTI,
}

impl CSRState {

    fn field_for(&mut self, address: usize) -> Result<&mut u32, TrapCause> {
        match address {
            addresses::MSTATUS | addresses::SSTATUS => Ok(&mut self.mstatus),
            addresses::MTVEC => Ok(&mut self.mtvec),
            addresses::MEPC => Ok(&mut self.mepc),
            addresses::MCAUSE => Ok(&mut self.mcause),
            addresses::MTVAL => Ok(&mut self.mtval),
            addresses::MCYCLE | addresses::CYCLE | addresses::TIME => Ok(&mut self.mcycle),
            addresses::MINSTRET | addresses::INSTRET => Ok(&mut self.minstret),
            addresses::MIP | addresses::SIP => Ok(&mut self.mip),
            addresses::MIE | addresses::SIE => Ok(&mut self.mie),
            addresses::MIDELEG => Ok(&mut self.mideleg),
            addresses::MEDELEG => Ok(&mut self.mideleg),
            _ => Err(TrapCause::IllegalInstruction { instruction: None }),
        }
    }

    pub fn read(&self, address: usize) -> Result<u32, TrapCause> {
        match address {
            addresses::MSTATUS => Ok(self.mstatus),
            addresses::MTVEC => Ok(self.mtvec),
            addresses::MEPC => Ok(self.mepc),
            addresses::MCAUSE => Ok(self.mcause),
            addresses::MTVAL => Ok(self.mtval),
            addresses::MCYCLE | addresses::CYCLE | addresses::TIME => Ok(self.mcycle),
            addresses::MINSTRET | addresses::INSTRET => Ok(self.minstret),
            addresses::MIP => Ok(self.mip),
            addresses::MIE => Ok(self.mie),
            addresses::MHARTID => Ok(0),
            addresses::SSTATUS => Ok(self.mstatus & masks::SSTATUS),
            addresses::SIP => Ok(self.mip & masks::SIP),
            addresses::SIE => Ok(self.mie & masks::PER_SOURCE_SIE),
            addresses::MIDELEG => Ok(self.mideleg),
            addresses::MEDELEG => Ok(self.mideleg),
            _ => Err(TrapCause::IllegalInstruction { instruction: None }),
        }
    }

    // "The top two bits (csr[11:10]) indicate whether the register is read/write (00, 01, or 10) or read-only (11).
    // The next two bits (csr[9:8]) encode the lowest privilege level that can access the CSR."
    // NOTE: 9:8 means, "In order to write to the CSR, you must have at least this much access"
    //  "Implementations will not raise an exception on writes of unsupported values 
    //   to a WARL field... Implementations can return any legal value on the read 
    //   of a WARL field when the last write was of an illegal value, 
    //   but the legal value returned should deterministically depend on the illegal 
    //   written value and the architectural state of the hart."
    // https://docs.riscv.org/reference/isa/_attachments/riscv-privileged.pdf
    pub fn guest_write(&mut self, address: usize, value: u32, current_mode: CPUMode) -> Result<u32, TrapCause> {
        let has_write_access = mask_and_shift(address as u32, 0b11 << ACCESS_TYPE_LOCATION) != READ_ONLY;
        let privilege_level = mask_and_shift(address as u32, 0b11 << MINIMUM_PRIVILEGE_LOCATION);
        let meets_minimum_privilege = privilege_level <= current_mode.as_privilege_level();
        if !has_write_access | !meets_minimum_privilege {
            // todo: encode more info about the specific trap failure?
            return Err(TrapCause::IllegalInstruction { instruction: None });
        }
        let property = self.field_for(address)?;

        match address {
            addresses::MIP => Ok(*property),
            addresses::SIP => Ok(*property & masks::SIP),
            addresses::SSTATUS => {
                let bits_to_write = value & masks::SSTATUS;
                let bits_minus_sstatus = *property & !masks::SSTATUS;
                let updated_mstatus = bits_minus_sstatus | bits_to_write;
                *property = updated_mstatus;
                Ok(*property & masks::SSTATUS)
            },
            addresses::SIE => {
                let bits_to_write = value & masks::PER_SOURCE_SIE;
                let bits_minus_sie = *property & !masks::PER_SOURCE_SIE;
                let updated_sie = bits_minus_sie | bits_to_write;
                *property = updated_sie;
                Ok(*property & masks::PER_SOURCE_SIE)
            },
            _ => {
                *property = value;
                Ok(value)
            }
        }
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

    pub fn update_mip_pending_bit(&mut self, location_id: MIPBits, location_value: u32) {
        match location_id {
            MIPBits::MTI => {
                // (mtime >= mtimecmp) as u32
                self.mip = set_bit_range(self.mip, location_value, 1, masks::MTI.trailing_zeros() as usize);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_cycle_increments_cycle_and_time_together() {
        let mut csr = build_csr_state();
        csr.update_cycle(CPUCycles::Cycle);
        assert_eq!(csr.read(addresses::CYCLE).unwrap(), 1);
        assert_eq!(csr.read(addresses::TIME).unwrap(), 1);
    }

    #[test]
    fn test_update_cycle_instret_tracks_its_own_count_not_cycles() {
        // cycle advances twice, then one instruction retires -- instret
        // should read 1 (its own first increment), not 3 (cycle's value).
        let mut csr = build_csr_state();
        csr.update_cycle(CPUCycles::Cycle);
        csr.update_cycle(CPUCycles::Cycle);
        csr.update_cycle(CPUCycles::Instret);
        assert_eq!(csr.read(addresses::INSTRET).unwrap(), 1);
    }

    #[test]
    fn test_csr_write_denies_insufficient_privilege() {
        // mepc (0x341) requires M -- writing from S should be rejected.
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(0x341, 42, CPUMode::S);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_csr_write_allows_sufficient_privilege() {
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(0x341, 42, CPUMode::M);
        assert!(outcome.is_ok());
    }
}
