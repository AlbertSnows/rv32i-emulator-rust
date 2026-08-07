// In RISC, the first two bits are reserved to discern 16bit from 32bit.
// We use 32 bit, so the first two should always be 11

use crate::fetcher::InstructionWord;




pub fn decode_word_to_instruction(raw_word: InstructionWord) -> String {
    // 0xB3 = 1011 0011
    // mask = 0111 1111 <- 7 bits
    // A bit "mask" is a sequence of bits used to separate other bits from one another
    // It's called a mask from the term masking tape. mask over that which you don't want changed.
    let mask = 0b0111_1111;
    // op code is 7 bits wide.
    // the mask will keep the first 7 bits, toss the rest.
    let opcode = raw_word.0 & mask;
    match opcode {
        ? => blah,
        _ => failure
    }


    let x = opcode.to_string();
    x
}