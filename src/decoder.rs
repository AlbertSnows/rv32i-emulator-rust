// In RISC, the first two bits are reserved to discern 16bit from 32bit.
// We use 32 bit, so the first two should always be 11

use crate::definitions::op_codes;
use crate::definitions::masks;
use crate::instructions::r::inst_r_add;
use crate::instructions::r::parse_r_inst;
use crate::instructions::Instruction;
use crate::utility::bit_operations::mask;
use crate::fetcher::InstructionWord;

pub fn decode_word_to_instruction(raw_word: InstructionWord) -> Result<Instruction, String> {
    // op code is 7 bits wide.
    // the mask will keep the first 7 bits, toss the rest.
    let opcode = mask(raw_word.0, masks::OP_CODE);
    match opcode {
        op_codes::LOAD => Ok(parse_i_inst(raw_word)), // todo: implement i type closure that takes op code type as first param?
        op_codes::ALU_IMM => Ok(parse_i_inst(raw_word)),
        op_codes::SYSTEM => Ok(parse_i_inst(raw_word)),
        op_codes::JALR => Ok(parse_i_inst(raw_word)),
        op_codes::R => Ok(parse_r_inst(raw_word)),
        op_codes::S => Ok(parse_s_inst(raw_word)),
        op_codes::B => Ok(parse_b_inst(raw_word)),
        op_codes::LUI => Ok(parse_u_inst(raw_word)),
        op_codes::AUIPC => Ok(parse_u_inst(raw_word)),
        op_codes::J => Ok(parse_j_inst(raw_word)),
        // b = binary format, # = signify binary format, 09 = output 9 total characters w/ 0 as padding
        _ => Err(format!("undefined opcode: {:#09b}", opcode))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_word_to_instruction() {
        use crate::instructions::r::AluOp;

        // add x3, x1, x2 -- confirms opcode dispatch routes to parse_r_inst
        // correctly; parse_r_inst's own funct3/funct7 logic is covered by
        // r.rs's tests, not re-tested here.
        let raw_word = InstructionWord(0x002081B3);
        let result = decode_word_to_instruction(raw_word);
        assert_eq!(result, Instruction::RType { op: AluOp::Add, rd: 3, rs1: 1, rs2: 2 });
    }

    #[test]
    fn test_decode_word_to_instruction_unrecognized_opcode_returns_err() {
        // 0b0000000 isn't any of the 10 real RV32I opcodes (doesn't even
        // end in 11), so this should hit the catch-all and return Err
        // rather than a decoded Instruction.
        let raw_word = InstructionWord(0b0000000);
        let result = decode_word_to_instruction(raw_word);
        assert!(result.is_err());
    }
}