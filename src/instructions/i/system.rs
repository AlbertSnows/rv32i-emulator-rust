use crate::definitions::cpu_definition::{RegisterFile, CPUState, PCState, CPUMode, CsrState};
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::definitions::codes::ExecutionSignal;
use crate::instructions::i::csr;
use crate::definitions::trap_cause::TrapCause;

#[derive(Debug, PartialEq)]
pub enum SystemOp {
    ECall,
    EBreak,
    MRet
}

pub fn parse_system_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
    let content = raw_word.0;
    // ecall/ebreak are funct3 = 000; every other funct3 under the SYSTEM
    // opcode is one of the six CSR instructions, handled in their own file.
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    if funct_three != 0 {
        return csr::parse_csr_inst(raw_word);
    }
    let distinguishing_bit = mask_and_shift(content, masks::BIT_TWENTY);
    let instruction_name = match distinguishing_bit {
        0 => SystemOp::ECall,
        1 => SystemOp::EBreak,
        _ => SystemOp::EBreak
    };
    Ok(Format::SystemType {
        op: instruction_name
    })
}


pub fn execute_i_system_type(op: &SystemOp, mode: &mut CPUMode, pc: &mut PCState, csr: &CsrState) -> Result<ExecutionSignal, TrapCause> {
    // let mode = cpu.mode;
    // do something based on mode? 
    // match mode {
    //     TrapCause::EnvironmentCallFromMMode => todo!(),
    //     TrapCause::EnvironmentCallFromSMode => todo!(),
    //     TrapCause::EnvironmentCallFromUMode => todo!(),
    // }
    match op {
        // todo: should this be err? is there a better way
        SystemOp::ECall => Err(TrapCause::EnvironmentCallFromMMode),
        SystemOp::EBreak => Err(TrapCause::Breakpoint),
        SystemOp::MRet => todo!()
    }
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
