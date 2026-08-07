// In RISC, the first two bits are reserved to discern 16bit from 32bit.
// We use 32 bit, so the first two should always be 11

use crate::definitions::op_codes;
use crate::definitions::masks;
use crate::instructions::r::inst_r_add;
use crate::instructions::r::parse_r_inst;
use crate::instructions::Instruction;
use crate::utility::bit_operations::mask;
use crate::fetcher::InstructionWord;

pub fn decode_word_to_instruction(raw_word: InstructionWord) -> Instruction {
    // op code is 7 bits wide.
    // the mask will keep the first 7 bits, toss the rest.
    let opcode = mask(raw_word.0, masks::OP_CODE);
    let word_parser = match opcode {
        op_codes::R => parse_r_inst,
        _ => panic!("undefined op code")
    };
    word_parser(raw_word)
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
    #[should_panic]
    fn test_decode_word_to_instruction_unimplemented_opcode_panics() {
        // LOAD's opcode -- a real, valid RV32I opcode, but decode only
        // handles R-type so far, so this should hit the catch-all and panic.
        let raw_word = InstructionWord(0b0000011);
        decode_word_to_instruction(raw_word);
    }
}