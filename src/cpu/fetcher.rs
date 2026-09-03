use crate::cpu::definitions::cpu::bus::{BASE_ADDRESS, BUSState, build_bus_state};
use crate::cpu::definitions::cpu::cpu_definition::{CPUMode, PCState, build_pc_state};
use crate::cpu::definitions::cpu::csr::CSRState;
use crate::cpu::definitions::cpu::memory::FULL_MEM_SIZE;
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::utility::types::ByteType;

// word in this context refers to the fixed-sized chunk of data to interface with
// words for us are 32bits of data that we'll have to decode later, for now it's just raw bits
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Instruction(pub u32, pub ByteType);

// This version is instructional, meant to show how you would manually construct the instruction word
pub fn fetch_word_from_memory_instructional(pc: &PCState, bus: &BUSState) -> Result<Instruction, TrapCause> {
    // RV32I does little endian by default. that means that even though bytes are stored first->last
    // the 32 bit int needs to be set up in the form last<-first
    // so, we have the range pc, pc+1, ...
    // but if we reverse it to pc+3, pc+2, ...
    // then we get little endian ordering by default, which makes constructing the word a bit easier.
    let pc_value = pc.read();
    if pc_value + 3 >= bus.ram.storage.len() {
        return Err(TrapCause::InstructionAccessFault { address: pc_value });
    }
    let pc_byte_range = (pc_value..(pc_value+4)).rev(); // Rev<Range<usize>>
    let byte_values_from_mem = pc_byte_range.map(|byte_index: usize| {
        bus.ram.storage[byte_index]
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


    Ok(Instruction(raw_word, ByteType::Word))
}

// Routes through bus.direct_read instead of touching storage directly 
// this is the fetch path (called by core.rs's perform_step every
// cycle), so it needs to go through the same routing/translation every
// other memory access goes through, not bypass it.
pub fn fetch_word_from_memory(pc: &PCState,
                              bus: &mut BUSState,
                              state: &CSRState,
                              mode: CPUMode) -> Result<Instruction, TrapCause> {
    let pc_value = pc.read();

    let raw_half_word_low = bus.guest_fetch(pc_value as u32,
                                   ByteType::HalfWord.as_num(),
                                   state,
                                   mode)?;
    let is_word = raw_half_word_low & 0b11 == 0b11;
    let instruction_word = if is_word {
        let raw_rest_high = bus.guest_fetch(
            (pc_value + 2) as u32,
            ByteType::HalfWord.as_num(),
            state,
            mode)?;
        Instruction(raw_half_word_low | (raw_rest_high << 16), ByteType::Word)
    } else {
        Instruction(raw_half_word_low, ByteType::HalfWord)
    };
    Ok(instruction_word)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::definitions::cpu::csr::build_csr_state;

    #[test]
    fn test_fetch_word_from_memory_instructional() {
        let pc = build_pc_state();
        let mut bus = build_bus_state();
        // ram storage is u8, so each index is 8 bits apart
        bus.ram.storage[0] = 1;
        bus.ram.storage[1] = 2;
        bus.ram.storage[2] = 3;
        bus.ram.storage[3] = 4;
        let outcome = fetch_word_from_memory_instructional(&pc, &bus);
        let expected_outcome = 0b0000_0100_0000_0011_0000_0010_0000_0001;
        assert_eq!(outcome.unwrap().0, expected_outcome);
    }

    #[test]
    fn test_fetch_word_from_memory() {
        let mut pc = build_pc_state();
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        pc.write(BASE_ADDRESS as usize);
        bus.ram.storage[0] = 3;
        bus.ram.storage[1] = 2;
        bus.ram.storage[2] = 3;
        bus.ram.storage[3] = 4;
        let expected_outcome = 0b0000_0100_0000_0011_0000_0010_0000_0011;
        assert_eq!(fetch_word_from_memory(&pc, &mut bus, &csr, CPUMode::M).unwrap().0, expected_outcome);
    }

    #[test]
    fn test_fetch_word_from_memory_out_of_bounds_returns_err() {
        let mut pc = build_pc_state();
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        // BASE_ADDRESS + FULL_MEM_SIZE - 2 means pc+3 reaches one past the
        // last valid index in ram.storage after translation
        //  should hit
        // the bounds check regardless of how big ram storage actually is.
        let addr = BASE_ADDRESS as usize + FULL_MEM_SIZE - 2;
        pc.write(addr);
        // low halfword's low 2 bits must be 11, or fetch_word_from_memory
        // decides this is a complete compressed instruction and never
        // attempts the second (out-of-bounds) halfword read at all.
        // direct_write (not raw storage indexing) translates the
        // BASE_ADDRESS-relative address correctly.
        bus.direct_write(addr, &[3]).unwrap();
        let outcome = fetch_word_from_memory(&pc, &mut bus, &csr, CPUMode::M);
        assert!(outcome.is_err());
    }

    #[test]
    fn test_fetch_word_from_memory_returns_half_word_for_compressed_instruction() {
        let mut pc = build_pc_state();
        let mut bus = build_bus_state();
        let csr = build_csr_state();
        pc.write(BASE_ADDRESS as usize);
        // low 2 bits of 0x0001 are 01, not 11, so this should be read as a
        // complete 2-byte compressed instruction, not the low half of a
        // wider one.
        bus.direct_write(BASE_ADDRESS as usize, &0x0001u16.to_le_bytes()).unwrap();
        let outcome = fetch_word_from_memory(&pc, &mut bus, &csr, CPUMode::M).unwrap();
        assert_eq!(outcome, Instruction(0x0001, ByteType::HalfWord));
    }
}

