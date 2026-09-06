use std::ops::RangeInclusive;
use crate::cpu::definitions::addresses;
pub(crate) use crate::cpu::definitions::addresses::CsrAddress;
use crate::cpu::definitions::codes::MISA_STATE;
use crate::cpu::definitions::cpu::cpu_definition::CPUMode;
use crate::cpu::definitions::masks;
use crate::cpu::definitions::masks::{CSR_ACCESS_TYPE, CSR_PRIVILEGE_LEVEL, MEIP, MSTATUS_TVM, MTI, SEIP};
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::utility::bit_operations::{mask_and_shift, set_bit_range};

// The top two bits (csr[11:10]) indicate whether the register is read/write (00, 01, or 10)
// or read-only (11).
const READ_ONLY: u32 = 0b11;

#[derive(Debug, Clone, PartialEq)]
pub enum CPUCycles {
    Cycle,
    Instret,
    Time
}

#[derive(Debug, Copy, PartialEq, Clone)]
pub struct CSRFlags {
    pub skip_instret_increment: bool
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
        minstreth: 0,
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
        flags: CSRFlags { skip_instret_increment: false },
        mcycleh: 0,
        tselect: 0,
        tdata1: 0,
        tdata2: 0,
        tcontrol: 0,
        mscratch: 0,
        mcounteren: 0,
        scounteren: 0,
        pmpcfg: [0; 4],
        pmpaddr: [0; 16],
        satp: 0,
        mstatush: 0,
    }
}

// CSR (Control and Status Register) address space is 12 bits wide (0..4096), per the Zicsr extension 
// separate storage from the general-purpose, not reg file
// This means that for a given address in CSR A, A is 12 bits wide.
#[derive(Debug, Copy, PartialEq, Clone)]
pub struct CSRState {
    // todo: look into bit flags, bit field, crate
    mstatus: u32,
    // mtvec has one extra bit, bit 0, that says how it jumps to the address
    // 0 = direct mode, 1 = vectored mode
    mtvec: u32,
    mepc: u32, 
    mcause: u32,
    mtval: u32,
    mcycle: u32,
    minstret: u32,
    minstreth: u32,
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
    pub flags: CSRFlags,
    mcycleh: u32,
    tselect: u32,
    tdata1: u32,
    tdata2: u32,
    tcontrol: u32,
    mscratch: u32,
    mcounteren: u32,
    scounteren: u32,
    pmpcfg: [u32; 4],
    pmpaddr: [u32; 16],
    satp: u32,
    mstatush: u32
}

impl CSRState {
    pub(crate) fn take_and_reset_instret_state(&mut self) -> bool {
        let was_set = self.flags.skip_instret_increment;
        self.flags.skip_instret_increment = false;
        was_set
    }
}

fn index_within_range(addr: CsrAddress, range: RangeInclusive<CsrAddress>) -> Option<usize> {
    let distance_from_start = addr.value().checked_sub(range.start().value())?;
    let range_span = range.end().value() - range.start().value();
    (distance_from_start <= range_span).then_some(distance_from_start as usize)
}


// Which interrupt-pending bit in mip is being updated.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InterruptSource {
    MTI,
    MEI,
    SEI
}


impl CSRState {


    // Merge only `mask`'s bits from `value` into `*property`, leaving every
    // other bit untouched, then return the property masked down to just
    // those bits -- the shape every "per-source" CSR write (MIP/SIP/
    // SSTATUS/SIE) shares: guest software only ever gets to touch its own
    // slice of a register the hardware/other privilege levels also use.
    fn write_masked(property: &mut u32, value: u32, mask: u32) -> u32 {
        let bits_to_write = value & mask;
        let preserved_bits = *property & !mask;
        *property = preserved_bits | bits_to_write;
        *property & mask
    }

    fn check_satp_access(&self, address: CsrAddress, current_mode: CPUMode) -> Result<(), TrapCause> {
        // When mstatus.TVM ("Trap Virtual Memory") is set, S-mode accessing
        // satp (or executing sfence.vma) must trap illegal-instruction
        // instead of succeeding -- this lets M-mode intercept and virtualize
        // address-translation management instead of giving S-mode direct
        // control over it.
        if address == addresses::SATP
            && current_mode == CPUMode::S
            && mask_and_shift(self.mstatus, MSTATUS_TVM) == 1
        {
            return Err(TrapCause::IllegalInstruction { instruction: None });
        }
        Ok(())
    }

    fn field_for(&mut self, address: CsrAddress) -> Result<&mut u32, TrapCause> {
        // Both PMP arrays (pmpcfg: 4 entries, pmpaddr: 16 entries) are addressed
        // as a contiguous CSR range starting at PMPCFG0/PMPADDR0. These two
        // helpers turn "is this address in range, and if so which array slot"
        // into one Option<usize> each, instead of that logic being duplicated
        // inline (once in field_for, once in read) as match guards.
        if let Some(idx) = index_within_range(address, addresses::PMPCFG0..=addresses::PMPCFG3) {
            return Ok(&mut self.pmpcfg[idx]);
        }
        if let Some(idx) = index_within_range(address, addresses::PMPADDR0..=addresses::PMPADDR15) {
            return Ok(&mut self.pmpaddr[idx]);
        }
        match address {
            addresses::MSTATUS | addresses::SSTATUS => Ok(&mut self.mstatus),
            addresses::MTVEC => Ok(&mut self.mtvec),
            addresses::MEPC => Ok(&mut self.mepc),
            addresses::MCAUSE => Ok(&mut self.mcause),
            addresses::MTVAL => Ok(&mut self.mtval),
            addresses::MCYCLE | addresses::CYCLE | addresses::TIME => Ok(&mut self.mcycle),
            addresses::MINSTRET | addresses::INSTRET => Ok(&mut self.minstret),
            addresses::MINSTRETH | addresses::INSTRETH=> Ok(&mut self.minstreth),
            addresses::MIP | addresses::SIP => Ok(&mut self.mip),
            addresses::MIE | addresses::SIE => Ok(&mut self.mie),
            addresses::MIDELEG => Ok(&mut self.mideleg),
            addresses::MEDELEG => Ok(&mut self.medeleg),
            addresses::SEPC => Ok(&mut self.sepc),
            addresses::SCAUSE => Ok(&mut self.scause),
            addresses::SSCRATCH => Ok(&mut self.sscratch),
            addresses::STVAL => Ok(&mut self.stval),
            addresses::STVEC => Ok(&mut self.stvec),
            addresses::MCYCLEH | addresses::CYCLEH | addresses::TIMEH => Ok(&mut self.mcycleh),
            addresses::TSELECT => Ok(&mut self.tselect),
            addresses::TDATA1 => Ok(&mut self.tdata1),
            addresses::TDATA2 => Ok(&mut self.tdata2),
            addresses::TCONTROL => Ok(&mut self.tcontrol),
            addresses::MSCRATCH => Ok(&mut self.mscratch),
            addresses::MCOUNTEREN => Ok(&mut self.mcounteren),
            addresses::SCOUNTNEREN => Ok(&mut self.scounteren),
            addresses::SATP => Ok(&mut self.satp),
            addresses::MSTATUSH => Ok(&mut self.mstatush),
            _ => Err(TrapCause::IllegalInstruction { instruction: None }),
        }
    }

    pub fn read(&self, address: CsrAddress, current_mode: CPUMode) -> Result<u32, TrapCause> {
        let privilege_level = mask_and_shift(address.value() as u32, CSR_PRIVILEGE_LEVEL);
        let meets_minimum_privilege = privilege_level <= current_mode.as_privilege_level();
        if !meets_minimum_privilege {
            return Err(TrapCause::IllegalInstruction { instruction: None });
        }
        if let Some(idx) = index_within_range(address, addresses::PMPCFG0..=addresses::PMPCFG3) {
            return Ok(self.pmpcfg[idx]);
        }
        if let Some(idx) = index_within_range(address, addresses::PMPADDR0..=addresses::PMPADDR15) {
            return Ok(self.pmpaddr[idx]);
        }
        match address {
            addresses::MSTATUS => Ok(self.mstatus),
            addresses::MTVEC => Ok(self.mtvec),
            addresses::MEPC => Ok(self.mepc),
            addresses::MCAUSE => Ok(self.mcause),
            addresses::MTVAL => Ok(self.mtval),
            addresses::MCYCLE | addresses::CYCLE | addresses::TIME => Ok(self.mcycle),
            addresses::MINSTRET | addresses::INSTRET => Ok(self.minstret),
            addresses::MINSTRETH | addresses::INSTRETH => Ok(self.minstreth),
            addresses::MCYCLEH | addresses::CYCLEH | addresses::TIMEH => Ok(self.mcycleh),

            addresses::MIP => Ok(self.mip),
            addresses::MIE => Ok(self.mie),
            addresses::MHARTID => Ok(0),
            addresses::MIDELEG => Ok(self.mideleg),
            addresses::MEDELEG => Ok(self.medeleg),

            addresses::SSTATUS => Ok(self.mstatus & masks::SSTATUS),
            addresses::SIP => Ok(self.mip & masks::SIP),
            addresses::SIE => Ok(self.mie & masks::PER_SOURCE_SIE),
            addresses::SEPC => Ok(self.sepc),
            addresses::SSCRATCH => Ok(self.sscratch),
            addresses::STVAL => Ok(self.stval),
            addresses::STVEC => Ok(self.stvec),
            addresses::SCAUSE => Ok(self.scause),

            addresses::MISA => Ok(MISA_STATE),
            addresses::MVENDORID => Ok(0),
            addresses::MARCHID => Ok(0),
            addresses::MIMPID => Ok(0),
            addresses::MSTATUSH => Ok(0),

            // Sdtrig (hardware trigger/breakpoint) CSRs. Hardcoded to 0
            // regardless of what was last written. 
            // This codebase has no
            // trigger-matching hardware (no comparator watching
            // fetch/load/store addresses against tdata2), so we declare
            // zero triggers rather than half-implement the mechanism.
            // This matters because the conformance test (rv32mi/breakpoint)
            // uses "did my write to tdata1 stick?" as its own probe for
            // "does this hardware actually support the trigger type I just
            // configured?".
            // if read() echoed the stored value back, the
            // test would conclude real trigger hardware exists and then
            // require it to actually fire a breakpoint exception, which we
            // can't do. Always reading 0 keeps that probe failing (in the
            // sense the test wants), which is what routes it to the
            // graceful "trigger type unsupported, skip" path instead.
            addresses::TSELECT => Ok(0),
            addresses::TDATA1 => Ok(0),
            addresses::TDATA2 => Ok(0),
            addresses::TCONTROL => Ok(0),

            addresses::MSCRATCH => Ok(self.mscratch),
            addresses::MCOUNTEREN => Ok(self.mcounteren),
            addresses::SCOUNTNEREN => Ok(self.scounteren),

            addresses::SATP => {
                self.check_satp_access(address, current_mode)?;
                Ok(self.satp)
            },
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
    pub fn guest_write(&mut self, address: CsrAddress, value: u32, current_mode: CPUMode) -> Result<u32, TrapCause> {
        let has_write_access = mask_and_shift(address.value() as u32, CSR_ACCESS_TYPE) != READ_ONLY;
        let privilege_level = mask_and_shift(address.value() as u32, CSR_PRIVILEGE_LEVEL);
        let meets_minimum_privilege = privilege_level <= current_mode.as_privilege_level();
        if !has_write_access | !meets_minimum_privilege {
            // todo: encode more info about the specific trap failure?
            return Err(TrapCause::IllegalInstruction { instruction: None });
        } else if address == addresses::MISA {
            return Ok(MISA_STATE);
        }

        // Do not double write to instret
        let is_instret = address == addresses::MINSTRET
            || address == addresses::INSTRET
            || address == addresses::MINSTRETH
            || address == addresses::INSTRETH;
        if is_instret {
            self.flags.skip_instret_increment = true;
        }

        self.check_satp_access(address, current_mode)?;

        let property = self.field_for(address)?;
        match address {
            addresses::MIP => Ok(Self::write_masked(property, value, masks::PER_SOURCE_MIP)),
            addresses::SIP => Ok(Self::write_masked(property, value, masks::SIP)),
            addresses::SSTATUS => Ok(Self::write_masked(property, value, masks::SSTATUS)),
            addresses::SIE => Ok(Self::write_masked(property, value, masks::PER_SOURCE_SIE)),
            addresses::STVEC | addresses::MTVEC => {
                // we do not support vectored mode, which is designated in [1:0], so
                // we force any writes to direct mode, 0b00
                *property = value & !0b11;
                Ok(*property)
            },
            addresses::MSTATUSH => Ok(*property),
            _ => {
                *property = value;
                Ok(value)
            }
        }
    }

    pub fn update_cycle(&mut self, cycle: CPUCycles) {
        match cycle {
            CPUCycles::Cycle | CPUCycles::Time => {
                let (new_val, did_wrap) = self.mcycle.overflowing_add(1);
                self.mcycle = new_val;
                if did_wrap {
                    self.mcycleh = self.mcycleh.wrapping_add(1);
                }
            },
            CPUCycles::Instret => {
                let (new_val, did_wrap) = self.minstret.overflowing_add(1);
                self.minstret = new_val;
                if did_wrap {
                    self.minstreth = self.minstreth.wrapping_add(1);
                }
            },
        }
    }

    pub fn set_interrupt_pending(&mut self, location_id: InterruptSource, is_pending: bool) {
        // MIP is machine interrupt pending
        // this function is used to handle updating specific kinds of interrupts
        let pending_num = is_pending as u32;
        match location_id {
            InterruptSource::MTI => {
                // (mtime >= mtimecmp) as u32
                self.mip = set_bit_range(self.mip, pending_num, 1, MTI.trailing_zeros() as usize);
            },
            InterruptSource::MEI => {
                self.mip = set_bit_range(self.mip, pending_num, 1, MEIP.trailing_zeros() as usize);
            },
            InterruptSource::SEI => {
                self.mip = set_bit_range(self.mip, pending_num, 1, SEIP.trailing_zeros() as usize);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_within_range_at_start_of_pmpcfg_range() {
        assert_eq!(index_within_range(addresses::PMPCFG0, addresses::PMPCFG0..=addresses::PMPCFG3), Some(0));
    }

    #[test]
    fn test_index_within_range_in_middle_of_pmpcfg_range() {
        let addr = CsrAddress::new(0x3A2).unwrap(); // PMPCFG0 + 2
        assert_eq!(index_within_range(addr, addresses::PMPCFG0..=addresses::PMPCFG3), Some(2));
    }

    #[test]
    fn test_index_within_range_at_end_of_pmpcfg_range() {
        assert_eq!(index_within_range(addresses::PMPCFG3, addresses::PMPCFG0..=addresses::PMPCFG3), Some(3));
    }

    #[test]
    fn test_index_within_range_just_past_end_of_pmpcfg_range_is_none() {
        let addr = CsrAddress::new(0x3A4).unwrap(); // PMPCFG3 + 1
        assert_eq!(index_within_range(addr, addresses::PMPCFG0..=addresses::PMPCFG3), None);
    }

    #[test]
    fn test_index_within_range_below_pmpcfg_range_is_none() {
        // MSTATUS (0x300) is a real CSR, but nowhere near the PMPCFG range --
        // checked_sub must return None here, not underflow/panic.
        assert_eq!(index_within_range(addresses::MSTATUS, addresses::PMPCFG0..=addresses::PMPCFG3), None);
    }

    #[test]
    fn test_index_within_range_at_start_of_pmpaddr_range() {
        assert_eq!(index_within_range(addresses::PMPADDR0, addresses::PMPADDR0..=addresses::PMPADDR15), Some(0));
    }

    #[test]
    fn test_index_within_range_in_middle_of_pmpaddr_range() {
        let addr = CsrAddress::new(0x3B8).unwrap(); // PMPADDR0 + 8
        assert_eq!(index_within_range(addr, addresses::PMPADDR0..=addresses::PMPADDR15), Some(8));
    }

    #[test]
    fn test_index_within_range_at_end_of_pmpaddr_range() {
        assert_eq!(index_within_range(addresses::PMPADDR15, addresses::PMPADDR0..=addresses::PMPADDR15), Some(15));
    }

    #[test]
    fn test_index_within_range_just_past_end_of_pmpaddr_range_is_none() {
        let addr = CsrAddress::new(0x3C0).unwrap(); // PMPADDR15 + 1
        assert_eq!(index_within_range(addr, addresses::PMPADDR0..=addresses::PMPADDR15), None);
    }

    #[test]
    fn test_index_within_range_below_pmpaddr_range_is_none() {
        assert_eq!(index_within_range(addresses::MSTATUS, addresses::PMPADDR0..=addresses::PMPADDR15), None);
    }

    #[test]
    fn test_update_cycle_increments_cycle_and_time_together() {
        let mut csr = build_csr_state();
        csr.update_cycle(CPUCycles::Cycle);
        assert_eq!(csr.read(addresses::CYCLE, CPUMode::M).unwrap(), 1);
        assert_eq!(csr.read(addresses::TIME, CPUMode::M).unwrap(), 1);
    }

    #[test]
    fn test_update_cycle_instret_tracks_its_own_count_not_cycles() {
        // cycle advances twice, then one instruction retires -- instret
        // should read 1 (its own first increment), not 3 (cycle's value).
        let mut csr = build_csr_state();
        csr.update_cycle(CPUCycles::Cycle);
        csr.update_cycle(CPUCycles::Cycle);
        csr.update_cycle(CPUCycles::Instret);
        assert_eq!(csr.read(addresses::INSTRET, CPUMode::M).unwrap(), 1);
    }

    #[test]
    fn test_csr_write_denies_insufficient_privilege() {
        // mepc (0x341) requires M -- writing from S should be rejected.
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(CsrAddress::new(0x341).unwrap(), 42, CPUMode::S);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_csr_write_allows_sufficient_privilege() {
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(CsrAddress::new(0x341).unwrap(), 42, CPUMode::M);
        assert!(outcome.is_ok());
    }

    #[test]
    fn test_guest_write_sepc_round_trips() {
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(addresses::SEPC, 100, CPUMode::M);
        assert!(outcome.is_ok());
        assert_eq!(csr.read(addresses::SEPC, CPUMode::M).unwrap(), 100);
    }

    #[test]
    fn test_guest_write_scause_round_trips() {
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(addresses::SCAUSE, 2, CPUMode::M);
        assert!(outcome.is_ok());
        assert_eq!(csr.read(addresses::SCAUSE, CPUMode::M).unwrap(), 2);
    }

    #[test]
    fn test_guest_write_stval_round_trips() {
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(addresses::STVAL, 0xDEAD_BEEF, CPUMode::M);
        assert!(outcome.is_ok());
        assert_eq!(csr.read(addresses::STVAL, CPUMode::M).unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn test_guest_write_stvec_round_trips() {
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(addresses::STVEC, 0x8000_0000, CPUMode::M);
        assert!(outcome.is_ok());
        assert_eq!(csr.read(addresses::STVEC, CPUMode::M).unwrap(), 0x8000_0000);
    }

    #[test]
    fn test_guest_write_sscratch_round_trips() {
        let mut csr = build_csr_state();
        let outcome = csr.guest_write(addresses::SSCRATCH, 0x1234, CPUMode::M);
        assert!(outcome.is_ok());
        assert_eq!(csr.read(addresses::SSCRATCH, CPUMode::M).unwrap(), 0x1234);
    }

    #[test]
    fn test_medeleg_and_mideleg_are_independent() {
        // medeleg and mideleg are separate registers -- writing one must
        // not affect the other. If both addresses alias the same backing
        // field, the second write below clobbers the first.
        let mut csr = build_csr_state();
        csr.guest_write(addresses::MEDELEG, 0b0000_0100, CPUMode::M).unwrap(); // delegate IllegalInstruction (bit 2)
        csr.guest_write(addresses::MIDELEG, 0b1000_0000, CPUMode::M).unwrap(); // delegate MTI (bit 7)
        assert_eq!(csr.read(addresses::MEDELEG, CPUMode::M).unwrap(), 0b0000_0100);
        assert_eq!(csr.read(addresses::MIDELEG, CPUMode::M).unwrap(), 0b1000_0000);
    }
}
