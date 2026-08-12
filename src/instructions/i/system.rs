use crate::definitions::cpu_definition::{RegisterFile, CPUState, PCState, CPUMode, CsrState};
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::utility::bit_operations::{ mask_and_shift, set_bit_range };
use crate::definitions::masks;
use crate::instructions::i::csr;
use crate::definitions::trap_cause::TrapCause;
use crate::definitions::codes::{ ExecutionSignal, MSTATUS, MEPC };

#[derive(Debug, PartialEq)]
pub enum SystemOp {
    ECall, // 0000000 00000 = 0x000
    EBreak, // 0000000 00001 = 0x001
    MRet // 0011000 00010 = 0x302
}

pub fn parse_system_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    // ecall/ebreak are funct3 = 000; every other funct3 under the SYSTEM
    // opcode is one of the six CSR instructions, handled in their own file.
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    if funct_three != 0 {
        return csr::parse_csr_inst(raw_word);
    }
    let distinguishing_bits = mask_and_shift(content, masks::CSR_ADDRESS);
    let instruction_name = match distinguishing_bits {
        0b000000000000 => Ok(SystemOp::ECall),
        0b000000000001 => Ok(SystemOp::EBreak),
        0b001100000010 => Ok(SystemOp::MRet),
        _ => Err(TrapCause::IllegalInstruction { instruction: Some(raw_word.0) })
    }?;
    Ok(Format::SystemType {
        op: instruction_name
    })
}


pub fn execute_i_system_type(op: &SystemOp, mode: &mut CPUMode, pc: &mut PCState, csr: &mut CsrState) -> Result<ExecutionSignal, TrapCause> {
    match op {
        // todo: should this be err? is there a better way
        SystemOp::ECall => Err(TrapCause::EnvironmentCallFromMMode),
        SystemOp::EBreak => Err(TrapCause::Breakpoint),
        SystemOp::MRet => inst_i_mret(mode, pc, csr)
    }
}

pub fn inst_i_mret(mode: &mut CPUMode, pc: &mut PCState, csr: &mut CsrState) -> Result<ExecutionSignal, TrapCause> {
    // "Attempting to execute an xRET instruction in a mode less privileged than x will raise an illegal-instruction exception."
    if *mode != CPUMode::M {
        return Err(TrapCause::IllegalInstruction { instruction: None });
    }
    // "xRET sets the pc to the value stored in the xepc register."
    pc.write(csr.read(MEPC) as usize); //

    let mstatus = csr.read(MSTATUS);
    // mpp is where the previous mode was stored. saved at (12:11), 
    let mpp = mask_and_shift(mstatus, masks::MPP);
    let mode_before_trap = CPUMode::from_privilege_level(mpp)?;
    // "Setting xPP to the least-privileged supported mode on an xRET helps identify software bugs in the management of the two-level privilege-mode stack"
    let mstatus_after_resetting_mpp = set_bit_range(mstatus, 00, 2, 11);
    //  "MRET then in mstatus/mstatush sets... MIE=MPIE, and MPIE=1"
    let mpie = mask_and_shift(mstatus, masks::MPIE);
    let mstatus_after_mie = set_bit_range(mstatus_after_resetting_mpp, mpie, 1, masks::MIE.trailing_zeros() as usize);
    let mstatus_after_mpie = set_bit_range(mstatus_after_mie, 1, 1, masks::MPIE.trailing_zeros() as usize);
    csr.write(MSTATUS, mstatus_after_mpie, CPUMode::M)?;
    // dereference assignment
    *mode = mode_before_trap;
    Ok(ExecutionSignal::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
