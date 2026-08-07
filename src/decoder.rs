// In RISC, the first two bits are reserved to discern 16bit from 32bit.
// We use 32 bit, so the first two should always be 11

use crate::fetcher::InstructionWord;
use crate::definitions::op_codes;
use crate::instructions::r::inst_r_add;

pub fn decode_word_to_instruction(raw_word: InstructionWord) -> String {
    // op code is 7 bits wide.
    // the mask will keep the first 7 bits, toss the rest.
    let opcode = mask(raw_word.0, OP_CODE_MASK);
    let word_parser = match opcode {
        op_codes::R => parse_r_inst,
        _ => "failure"
    };

    


    let x = opcode.to_string();
    x
    // return (correct function, parsed )
}