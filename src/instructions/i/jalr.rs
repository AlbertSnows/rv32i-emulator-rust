use crate::definitions::cpu_definition::RegisterFile;
use crate::definitions::cpu_definition::PCState;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::utility::bit_operations::shake_to_signed;

// jalr -- the only instruction under its opcode, so no op enum needed
// (same reasoning as JType). Not yet implemented.

pub fn parse_jalr_inst(raw_word: InstructionWord) -> Result<Format, String> {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let imm_unsigned = mask_and_shift(content, masks::I_TYPE_JALR);
    let imm_val = shake_to_signed(imm_unsigned, 12);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    Ok(Format::JalrType {
        imm: imm_val,
        rd: reg_dest as usize,
        rs1: reg_source_one as usize
    })
}

pub fn execute_i_jalr_type(rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile, pc: &PCState) -> Result<ExecutionSignal, String> {
    inst_i_jalr();
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_jalr() {
    // rd <- pc+4
    reg_file[pd] = pc.read() + 4;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jalr_inst() {
        // jalr x1, x2, 8 -- opcode = 1100111 (JALR), rd = 1, rs1 = 2, imm = 8
        let raw_word = InstructionWord(0x008100E7);
        let result = parse_jalr_inst(raw_word);
        assert_eq!(result, Ok(Format::JalrType { rd: 1, rs1: 2, imm: 8 }));
    }

    #[test]
    fn test_inst_i_jalr() {
        let rd = 1;
        let pc = build_pc();
        let reg_file = build_register_file();
        pc.write(3);
        execute_i_jalr_type();
        assert_eq!(reg_file.read(1), 7);
    }
}