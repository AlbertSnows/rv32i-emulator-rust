use crate::fetcher::InstructionWord;
fn decode_word_to_instruction(raw_word: InstructionWord) {
    // 0xB3 = 1011 0011
    // mask = 0111 1111 <- 7 bits
    // A bit "mask" is a sequence of bits used to separate other bits from one another
    // It's called a mask from the term masking tape. mask over that which you don't want changed.
    let mask = 0b0111_1111;
    // op code is 7 bits wide.
    let opcode = raw_word.0 & mask;
}