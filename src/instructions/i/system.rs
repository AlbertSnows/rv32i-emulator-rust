use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::definitions::codes::ExecutionSignal;

#[derive(Debug, PartialEq)]
pub enum SystemOp {
    ECall,
    EBreak
}

#[derive(Debug, PartialEq)]
pub enum CsrType {
    Csrrw,
    Csrrs,
    Csrrc,
    Csrrwi,
    Csrrsi,
    Csrrci
}

pub fn parse_system_inst(raw_word: InstructionWord) -> Result<Format, String> {
    let content = raw_word.0;
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


pub fn execute_i_system_type(op: &SystemOp) -> Result<ExecutionSignal, String> {
    match op {
        SystemOp::ECall => Ok(ExecutionSignal::Continue),
        SystemOp::EBreak => Ok(ExecutionSignal::Halt)
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
