
use crate::cpu_definition::PCState;
use crate::cpu_definition::MemoryState;
use crate::cpu_definition::build_memory_state;
use crate::cpu_definition::build_pc_state;
// word in this context refers to the fixed-sized chunk of data to interface with
// words for us are 32bits of data that we'll have to decode later, for now it's just raw bits
pub struct InstructionWord(pub u32);

pub fn fetch_word_from_memory(pc: &PCState, mem: &MemoryState) -> InstructionWord {
    // RV32I does little endian by default. that means that even though bytes are stored first->last
    // the 32 bit int needs to be set up in the form last<-first
    // so, we have the range pc, pc+1, ... 
    // but if we reverse it to pc+3, pc+2, ...
    // then we get little endian ordering by default, which makes constructing the word a bit easier. 
    let pc_byte_range = (pc.value..(pc.value+4)).rev(); // Rev<Range<usize>>
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


    InstructionWord(raw_word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_word_from_memory() {
        let pc = build_pc_state();
        let mut mem = build_memory_state(); 
        // mem is u8, so each index is 8 bits apart
        mem.storage[0] = 1;
        mem.storage[1] = 2;
        mem.storage[2] = 3;
        mem.storage[3] = 4;
        let outcome = fetch_word_from_memory(&pc, &mem);
        let expected_outcome = 0b0000_0100_0000_0011_0000_0010_0000_0001;
        assert_eq!(outcome.0, expected_outcome);
    }
}

