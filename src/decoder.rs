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