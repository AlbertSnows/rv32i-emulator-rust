use crate::definitions::trap_cause::TrapCause;
use crate::definitions::addresses::{MTIME, MTIMECMP, MTIME_END, MTIMECMP_END};
use crate::utility::bit_operations::{ as_window, extract_sub_bytes };
use crate::utility::types::{ ByteType, as_byte_type };

const TEST_MEM_SIZE: usize = 4096;
const FULL_MEM_SIZE: usize = 65536;
#[derive(Debug, Copy, PartialEq, Clone)]
pub struct MemoryState {
    pub storage: [u8; TEST_MEM_SIZE],
    pub mtime: u64,
    pub mtimecmp: u64,
}

pub fn build_memory_state() -> MemoryState {
    MemoryState { 
        storage: [0; TEST_MEM_SIZE],
        mtime: 0,
        mtimecmp: 0
    }
}

impl MemoryState {

    pub fn update_time(&mut self) {
        self.mtime += 1;
    }
    
    // Writes the low `num_bytes` bytes of `value` to memory starting at
    // `address`, little-endian (least-significant byte at the lowest address)
    // same ordering fetch_word_from_memory reads back.
    // It does not worry about how many bytes to write, that's the responsibility
    // of the parent
    pub fn write_bytes(&mut self, address: usize, bytes: &[u8]) -> Result<(), TrapCause> {
        if address + bytes.len() > self.storage.len() {
            return Err(TrapCause::StoreAccessFault { address });
        }
        match address {
            MTIME..=MTIME_END => {
                let offset = address - MTIME; 
                let mut mtime_bytes = self.mtime.to_le_bytes();
                let update_range = offset..(offset + bytes.len());
                mtime_bytes[update_range].copy_from_slice(bytes);
                self.mtime = u64::from_le_bytes(mtime_bytes);
                Ok(())
            },
            MTIMECMP..=MTIMECMP_END => {
                let offset = address - MTIMECMP; 
                let mut mtimecmp_bytes = self.mtimecmp.to_le_bytes();
                let update_range = offset..(offset + bytes.len());
                mtimecmp_bytes[update_range].copy_from_slice(bytes);
                self.mtimecmp = u64::from_le_bytes(mtimecmp_bytes);
                Ok(())
            },
            _ => {
                bytes.iter().enumerate().for_each(|(i, byte)| {
                    self.storage[address + i] = *byte;
                });
                Ok(())
            }
        }
    }

    // Reads `num_bytes` bytes starting at `address` and combines them
    // little-endian (lowest address = least-significant byte) into a u32,
    // the mirror image of write_bytes.
    pub fn read_bytes(&self, address: usize, num_bytes: usize) -> Result<u32, TrapCause> {
        if address + num_bytes > self.storage.len() {
            return Err(TrapCause::LoadAccessFault { address });
        }
        match address {
            // X..=Y means range X to Y, inclusive Y
            MTIME..=MTIME_END => {
                let offset = address - MTIME;
                let width = as_byte_type(num_bytes).ok_or(TrapCause::LoadAccessFault { address })?;
                let sub_bytes = extract_sub_bytes(self.mtime, offset, width);
                Ok(sub_bytes as u32)
            },
            MTIMECMP..=MTIMECMP_END => {
                let offset = address - MTIMECMP;
                let width = as_byte_type(num_bytes).ok_or(TrapCause::LoadAccessFault { address })?;
                let sub_bytes = extract_sub_bytes(self.mtimecmp, offset, width);
                Ok(sub_bytes as u32)            },
            _ => {
                let byte_range = 0..num_bytes;
                let value = byte_range.fold(0u32, |acc, i| {
                    let byte_at_index = self.storage[address + i] as u32;
                    let positioned_byte = byte_at_index << (i * 8);
                    acc | positioned_byte
                });
                Ok(value)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_write_bytes_single_byte() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0b0101_1010;
        mem.write_bytes(8, &test_value.to_le_bytes()).unwrap();
        assert_eq!(mem.storage[8], 0b0101_1010);
    }

    #[test]
    fn test_write_bytes_halfword_little_endian() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0b1111_0000_1010_0101;
        mem.write_bytes(8, &test_value.to_le_bytes()).unwrap();
        assert_eq!(mem.storage[8], 0b1010_0101);
        assert_eq!(mem.storage[9], 0b1111_0000);
    }

    #[test]
    fn test_write_bytes_word_little_endian() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0x12345678;
        mem.write_bytes(8, &test_value.to_le_bytes()).unwrap();
        assert_eq!(mem.storage[8], 0x78);
        assert_eq!(mem.storage[9], 0x56);
        assert_eq!(mem.storage[10], 0x34);
        assert_eq!(mem.storage[11], 0x12);
    }

    #[test]
    fn test_write_bytes_out_of_bounds_returns_err() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0x12345678;
        let outcome = mem.write_bytes(TEST_MEM_SIZE - 2, &test_value.to_le_bytes());
        assert!(outcome.is_err());
    }

    #[test]
    fn test_read_bytes_single_byte() {
        let mut mem = build_memory_state();
        mem.storage[8] = 0b0101_1010;
        assert_eq!(mem.read_bytes(8, 1).unwrap(), 0b0101_1010);
    }

    #[test]
    fn test_read_bytes_halfword_little_endian() {
        let mut mem = build_memory_state();
        mem.storage[8] = 0b1010_0101;
        mem.storage[9] = 0b1111_0000;
        assert_eq!(mem.read_bytes(8, 2).unwrap(), 0b1111_0000_1010_0101);
    }

    #[test]
    fn test_read_bytes_word_little_endian() {
        let mut mem = build_memory_state();
        mem.storage[8] = 0x78;
        mem.storage[9] = 0x56;
        mem.storage[10] = 0x34;
        mem.storage[11] = 0x12;
        assert_eq!(mem.read_bytes(8, 4).unwrap(), 0x12345678);
    }

    #[test]
    fn test_read_bytes_out_of_bounds_returns_err() {
        let mem = build_memory_state();
        let outcome = mem.read_bytes(TEST_MEM_SIZE - 2, 4);
        assert!(outcome.is_err());
    }
}