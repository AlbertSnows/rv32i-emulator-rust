
use crate::definitions::cpu_definition::{PCState, MemoryState, build_memory_state, build_pc_state};
use crate::definitions::trap_cause::TrapCause;
// word in this context refers to the fixed-sized chunk of data to interface with
// words for us are 32bits of data that we'll have to decode later, for now it's just raw bits
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct InstructionWord(pub u32);

// This version is instructional, meant to show how you would manually construct the instruction word
pub fn fetch_word_from_memory_instructional(pc: &PCState, mem: &MemoryState) -> Result<InstructionWord, TrapCause> {
    // RV32I does little endian by default. that means that even though bytes are stored first->last
    // the 32 bit int needs to be set up in the form last<-first
    // so, we have the range pc, pc+1, ...
    // but if we reverse it to pc+3, pc+2, ...
    // then we get little endian ordering by default, which makes constructing the word a bit easier.
    let pc_value = pc.read();
    if pc_value + 3 >= mem.storage.len() {
        return Err(TrapCause::InstructionAccessFault { address: pc_value });
    }
    let pc_byte_range = (pc_value..(pc_value+4)).rev(); // Rev<Range<usize>>
    let byte_values_from_mem = pc_byte_range.map(|byte_index: usize| {
        mem.storage[byte_index]
    }); // effectively holds a Vec<u8>, can also be thought of as [u8; 4]
    // Now suppose we have 00, 20, 81, B3
    // we start with 0x000...
    // we want 0x002081B3
    // x << 8 literally shifts bits left<-right (010<-001) by 8
    // | = OR, X | Y for u32 is OR'ing them together.
    // so if X = 00 00 20 00 and Y = 00 00 00 81 then X | Y is 00 00 20 81
    // repeat the bit shift, and we have 00 20 81 00, repeat
    // thus, starting with the left most value, we build right to left, pushing our leftmost value
    // towards its final spot by the final iteration.
    let raw_word: u32 = byte_values_from_mem.fold(0, |raw_word_acc: u32, byte| {
        let updated_acc = (raw_word_acc << 8) | byte as u32;
        updated_acc
    });


    Ok(InstructionWord(raw_word))
}

// Instead of the manual reverse-range-and-fold, mem.storage is already laid
// out little-endian (storage[pc] is the least-significant byte)
// The four bytes can be handed to from_le_bytes in forward order
// Does not require shifting
pub fn fetch_word_from_memory(pc: &PCState, mem: &MemoryState) -> Result<InstructionWord, TrapCause> {
    let pc_value = pc.read();
    if pc_value + 3 >= mem.storage.len() {
        return Err(TrapCause::InstructionAccessFault { address: pc_value });
    }
    let bytes = [
        mem.storage[pc_value],
        mem.storage[pc_value + 1],
        mem.storage[pc_value + 2],
        mem.storage[pc_value + 3],
    ];
    Ok(InstructionWord(u32::from_le_bytes(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_word_from_memory_instructional() {
        let pc = build_pc_state();
        let mut mem = build_memory_state();
        // mem is u8, so each index is 8 bits apart
        mem.storage[0] = 1;
        mem.storage[1] = 2;
        mem.storage[2] = 3;
        mem.storage[3] = 4;
        let outcome = fetch_word_from_memory_instructional(&pc, &mem);
        let expected_outcome = 0b0000_0100_0000_0011_0000_0010_0000_0001;
        assert_eq!(outcome.unwrap().0, expected_outcome);
    }

    #[test]
    fn test_fetch_word_from_memory() {
        let pc = build_pc_state();
        let mut mem = build_memory_state();
        mem.storage[0] = 1;
        mem.storage[1] = 2;
        mem.storage[2] = 3;
        mem.storage[3] = 4;
        let expected_outcome = 0b0000_0100_0000_0011_0000_0010_0000_0001;
        assert_eq!(fetch_word_from_memory(&pc, &mem).unwrap().0, expected_outcome);
    }

    #[test]
    fn test_fetch_word_from_memory_out_of_bounds_returns_err() {
        let mut pc = build_pc_state();
        let mem = build_memory_state();
        // mem.storage is 4096 bytes (indices 0..4095); pc=4094 means pc+3=4097,
        // which reaches past the last valid index -- should hit the bounds check.
        pc.write(4094);
        let outcome = fetch_word_from_memory(&pc, &mem);
        assert!(outcome.is_err());
    }
}

