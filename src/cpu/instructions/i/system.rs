use crate::cpu::definitions::cpu::cpu_definition::{RegisterFile, CPUState, PCState, CPUMode};
use crate::cpu::fetcher::InstructionWord;
use crate::cpu::instructions::Format;
use crate::cpu::utility::bit_operations::{mask_and_shift, set_bit_range };
use crate::cpu::definitions::masks;
use crate::cpu::instructions::i::csr;
use crate::cpu::definitions::trap_cause::{TrapCause, TrapDestination, M_TRAP, S_TRAP};
use crate::cpu::definitions::codes::{ ExecutionSignal };
use crate::cpu::definitions::addresses::{MSTATUS, MEPC, SEPC, SSTATUS};
use crate::cpu::definitions::cpu::csr::CSRState;
use crate::cpu::definitions::masks::{MSTATUS_TVM, MSTATUS_TSR};

#[derive(Debug, PartialEq)]
pub enum SystemOp {
    ECall, // 0000000 00000 = 0x000
    EBreak, // 0000000 00001 = 0x001
    MRet, // 0011000 00010 = 0x302
    WFI,
    SRet,
    SFenceVma,
}

pub fn parse_system_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    // ecall/ebreak are funct3 = 000; every other funct3 under the SYSTEM
    // opcode is one of the six CSR instructions, handled in their own file.
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    if funct_three != 0 {
        return csr::parse_csr_inst(raw_word);
    }
    let funct_seven = mask_and_shift(content, masks::FUNCT_SEVEN);
    if funct_seven == 0b0001001 {
        return Ok(Format::SystemType { op: SystemOp::SFenceVma });
    }
    let distinguishing_bits = mask_and_shift(content, masks::CSR_ADDRESS);
    let instruction_name = match distinguishing_bits {
        0b000000000000 => Ok(SystemOp::ECall),
        0b000000000001 => Ok(SystemOp::EBreak),
        0b001100000010 => Ok(SystemOp::MRet),
        0b0001_0000_0010 => Ok(SystemOp::SRet),
        0b000100000101 => Ok(SystemOp::WFI),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(raw_word.0) })
    }?;
    Ok(Format::SystemType {
        op: instruction_name
    })
}


pub fn execute_i_system_type(op: &SystemOp, cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    match op {
        // todo: should this be err? is there a better way
        SystemOp::ECall => match cpu.mode {
            CPUMode::M => Err(TrapCause::EnvironmentCallFromMMode),
            CPUMode::S => Err(TrapCause::EnvironmentCallFromSMode),
            CPUMode::U => Err(TrapCause::EnvironmentCallFromUMode),
        },
        SystemOp::EBreak => Err(TrapCause::Breakpoint),
        SystemOp::MRet => inst_i_xret(cpu, &M_TRAP),
        SystemOp::SRet => {
            if cpu.mode == CPUMode::S && mask_and_shift(cpu.csr.read(MSTATUS, CPUMode::M)?, MSTATUS_TSR) == 1 {
                return Err(TrapCause::IllegalInstruction { instruction: None });
            }
            inst_i_xret(cpu, &S_TRAP)
        },
        SystemOp::WFI => Ok(ExecutionSignal::Continue),
        SystemOp::SFenceVma => {
            if cpu.mode == CPUMode::S && mask_and_shift(cpu.csr.read(MSTATUS, CPUMode::M)?, MSTATUS_TVM) == 1 {
                return Err(TrapCause::IllegalInstruction { instruction: None })
            }
            Ok(ExecutionSignal::Continue)
        }
    }
}

// essentially the inverse of handle_trap
// trap function needs to run inst_i_mret to return cpu to normal state
pub fn inst_i_xret(cpu: &mut CPUState, dest: &TrapDestination) -> Result<ExecutionSignal, TrapCause> {
    let mode = &mut cpu.mode;
    let pc = &mut cpu.pc;
    let csr = &mut cpu.csr;
    // "Attempting to execute an xRET instruction in a mode less privileged than x will raise an illegal-instruction exception."
    if mode.as_privilege_level() < dest.mode.as_privilege_level() {
        return Err(TrapCause::IllegalInstruction { instruction: None });
    }
    // "xRET sets the pc to the value stored in the xepc register."
    let epc_val = csr.read(dest.epc, *mode)? as usize;
    pc.write(epc_val); //

    let mstatus = csr.read(MSTATUS, CPUMode::M)?;
    // mpp is where the previous mode was stored. saved at (12:11), 
    let pp_val = mask_and_shift(mstatus, dest.pp_mask);
    let mode_before_trap = CPUMode::from_privilege_level(pp_val)?;
    let pp_width = dest.pp_mask.count_ones() as usize;
    let pp_position = dest.pp_mask.trailing_zeros() as usize;
    // "Setting xPP to the least-privileged supported mode on an xRET helps identify software bugs in the management of the two-level privilege-mode stack"
    let mstatus_after_resetting_mpp = set_bit_range(mstatus, 00, pp_width, pp_position);
    //  "MRET then in mstatus/mstatush sets... MIE=MPIE, and MPIE=1"
    let pie_val = mask_and_shift(mstatus, dest.pie_mask);
    let mstatus_after_ie = set_bit_range(mstatus_after_resetting_mpp, pie_val, 1, dest.ie_mask.trailing_zeros() as usize);
    let mstatus_after_pie = set_bit_range(mstatus_after_ie, 1, 1, dest.pie_mask.trailing_zeros() as usize);
    csr.guest_write(MSTATUS, mstatus_after_pie, CPUMode::M)?;
    // dereference assignment
    *mode = mode_before_trap;
    cpu.flags.in_trap = false;
    Ok(ExecutionSignal::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::definitions::cpu::cpu_definition::build_cpu_state;

    #[test]
    fn test_parse_system_inst_ecall() {
        // ecall -- bit 20 (the ecall/ebreak discriminator) = 0
        let raw_word = InstructionWord(0x00000073);
        let result = parse_system_inst(raw_word);
        assert_eq!(result, Ok(Format::SystemType { op: SystemOp::ECall }));
    }

    #[test]
    fn test_parse_system_inst_ebreak() {
        // ebreak -- bit 20 = 1
        let raw_word = InstructionWord(0x00100073);
        let result = parse_system_inst(raw_word);
        assert_eq!(result, Ok(Format::SystemType { op: SystemOp::EBreak }));
    }

    #[test]
    fn test_parse_system_inst_mret() {
        // mret -- funct12 = 0x302 (0011_0000_0010), rs1/rd/funct3 all zero
        let raw_word = InstructionWord(0x30200073);
        let result = parse_system_inst(raw_word);
        assert_eq!(result, Ok(Format::SystemType { op: SystemOp::MRet }));
    }

    #[test]
    fn test_parse_system_inst_sret() {
        // sret -- funct12 = 0x102 (0001_0000_0010), rs1/rd/funct3 all zero
        let raw_word = InstructionWord(0x10200073);
        let result = parse_system_inst(raw_word);
        assert_eq!(result, Ok(Format::SystemType { op: SystemOp::SRet }));
    }

    #[test]
    fn test_inst_i_mret_illegal_when_not_m() {
        let mut cpu = build_cpu_state();
        cpu.mode = CPUMode::S;
        let outcome = inst_i_xret(&mut cpu, &M_TRAP);
        assert_eq!(outcome, Err(TrapCause::IllegalInstruction { instruction: None }));
        // nothing should have changed on the illegal path
        assert_eq!(cpu.mode, CPUMode::S);
        assert_eq!(cpu.pc.read(), 0);
    }

    #[test]
    fn test_inst_i_mret_restores_pc_and_mode() {
        let mut cpu = build_cpu_state();
        cpu.csr.guest_write(MEPC, 100, CPUMode::M);
        // MPP = S (level 1) at bits 11-12
        cpu.csr.guest_write(MSTATUS, 1 << 11, CPUMode::M);

        let outcome = inst_i_xret(&mut cpu, &M_TRAP);

        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
        assert_eq!(cpu.pc.read(), 100);
        assert_eq!(cpu.mode, CPUMode::S);
        // MPP should be reset to 0 (U) afterward
        assert_eq!(mask_and_shift(cpu.csr.read(MSTATUS, CPUMode::M).unwrap(), masks::MPP), 0);
    }

    #[test]
    fn test_inst_i_mret_clears_in_trap_flag() {
        let mut cpu = build_cpu_state();
        cpu.flags.in_trap = true;
        let outcome = inst_i_xret(&mut cpu, &M_TRAP);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
        assert_eq!(cpu.flags.in_trap, false);
    }

    #[test]
    fn test_inst_i_sret_illegal_below_s() {
        // SRET is illegal from U -- less privileged than S. Unlike MRET
        // (legal only from M), SRET is legal from S *or* M.
        let mut cpu = build_cpu_state();
        cpu.mode = CPUMode::U;
        let outcome = inst_i_xret(&mut cpu, &S_TRAP);
        assert_eq!(outcome.err().unwrap(), TrapCause::IllegalInstruction { instruction: None });
        assert_eq!(cpu.mode, CPUMode::U);
        assert_eq!(cpu.pc.read(), 0);

    }

    #[test]
    fn test_inst_i_sret_restores_pc_and_mode() {
        let mut cpu = build_cpu_state();
        cpu.mode = CPUMode::S; // SRET is legal from S (or M)
        cpu.csr.guest_write(SEPC, 100, CPUMode::M).expect("sepc write defined");
        // SPP = U (0) at bit 8 -- the trapping mode was U
        cpu.csr.guest_write(MSTATUS, 0 << 8, CPUMode::M).expect("write should succeed");

        let outcome = inst_i_xret(&mut cpu, &S_TRAP);
        assert_eq!(outcome.unwrap(), ExecutionSignal::Continue);
        assert_eq!(cpu.mode, CPUMode::U);
        assert_eq!(cpu.pc.read(), 100);
        assert_eq!(mask_and_shift(cpu.csr.read(MSTATUS, CPUMode::M).unwrap(), masks::SPP), 0);
    }

    #[test]
    fn test_inst_i_sret_clears_in_trap_flag() {
        let mut cpu = build_cpu_state();
        cpu.mode = CPUMode::S;
        cpu.flags.in_trap = true;
        let outcome = inst_i_xret(&mut cpu, &S_TRAP);
        assert_eq!(outcome.unwrap(), ExecutionSignal::Continue);
        assert_eq!(cpu.flags.in_trap, false);
    }

}
