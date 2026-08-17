use crate::definitions::cpu::memory::{MemoryState, build_memory_state};
use crate::definitions::trap_cause::{TrapCause};
use crate::definitions::addresses::{MTIME, MTIMECMP, MTIME_END, MTIMECMP_END};
use crate::utility::types::{ ByteType, as_byte_type };
use crate::utility::bit_operations::{ as_window, extract_sub_bytes };
// every riscv-tests binary links at exactly this address. 
// The low half of the 32-bit address
// space is reserved for boot ROM and memory-mapped
// peripherals e.g. CLINT (mtime/mtimecmp). 
// mem.storage is only FULL_MEM_SIZE bytes, nowhere near big enough to
// use 0x80000000 directly as an array index, so BUSState translates:
// any address >= BASE_ADDRESS gets BASE_ADDRESS subtracted before it
// reaches self.ram. MemoryState itself never sees this
// constant.
pub const BASE_ADDRESS: u32 = 0x8000_0000;

#[derive(Debug, PartialEq, Clone)]
pub struct BUSState {
    pub ram: MemoryState,
    pub clint: ClintState
}

pub fn build_bus_state() -> BUSState {
    BUSState {
        ram: build_memory_state(),
        clint: ClintState { mtime: 0, mtimecmp: 0 }
    }
}

impl BUSState {
    pub fn direct_read(&self, address: usize, num_bytes: usize) -> Result<u32, TrapCause> {
        match address {
            // X..=Y means range X to Y, inclusive Y
            MTIME..=MTIME_END => {
                let mtime_offset = address - MTIME;
                self.clint.read_mtime(mtime_offset, num_bytes)
            },
            MTIMECMP..=MTIMECMP_END => {
                let mtimecmp_offset = address - MTIMECMP;
                self.clint.read_mtimecmp(mtimecmp_offset, num_bytes)
            },
            _ => {
                if address < BASE_ADDRESS as usize {
                    return Err(TrapCause::LoadAccessFault { address });
                }
                let real_index = address - BASE_ADDRESS as usize;
                self.ram.read_bytes(real_index, num_bytes)
            }
        }
    }
    pub fn direct_write(&mut self, address: usize, bytes: &[u8]) -> Result<(), TrapCause> {
        match address {
            MTIME..=MTIME_END => {
                let mtime_offset = address - MTIME;
                self.clint.write_mtime(mtime_offset, bytes)
            },
            MTIMECMP..=MTIMECMP_END => {
                let mtimecmp_offset = address - MTIMECMP;
                self.clint.write_mtimecmp(mtimecmp_offset, bytes)
            },
            _ => {
                if address < BASE_ADDRESS as usize {
                    return Err(TrapCause::LoadAccessFault { address });
                }
                let real_index = address - BASE_ADDRESS as usize;
                self.ram.write_bytes(real_index, bytes)
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ClintState {
    pub mtime: u64,
    pub mtimecmp: u64,
}

impl ClintState {
    pub fn update_time(&mut self) {
        self.mtime += 1;
    }

    pub fn read_mtime(&self, offset: usize, num_bytes: usize) -> Result<u32, TrapCause> {
        Self::read_register(self.mtime, offset, num_bytes)
    }

    pub fn read_mtimecmp(&self, offset: usize, num_bytes: usize) -> Result<u32, TrapCause> {
        Self::read_register(self.mtimecmp, offset, num_bytes)
    }

    pub fn write_mtime(&mut self, offset: usize, bytes: &[u8]) -> Result<(), TrapCause> {
        self.mtime = Self::write_register(self.mtime, offset, bytes)?;
        Ok(())
    }

    pub fn write_mtimecmp(&mut self, offset: usize, bytes: &[u8]) -> Result<(), TrapCause> {
        self.mtimecmp = Self::write_register(self.mtimecmp, offset, bytes)?;
        Ok(())
    }

    // mtime/mtimecmp are each exactly 8 real bytes (DoubleWord) 
    // offset + num_bytes has to stay within that, or extract_sub_bytes would
    // read past the end of a fixed 8-byte array and panic instead of
    // returning a clean error.
    fn read_register(register_value: u64, offset: usize, num_bytes: usize) -> Result<u32, TrapCause> {
        if offset + num_bytes > 8 {
            return Err(TrapCause::LoadAccessFault { address: offset });
        }
        let width = as_byte_type(num_bytes).ok_or(TrapCause::LoadAccessFault { address: offset })?;
        Ok(extract_sub_bytes(register_value, offset, width) as u32)
    }

    fn write_register(register_value: u64, offset: usize, bytes: &[u8]) -> Result<u64, TrapCause> {
        if offset + bytes.len() > 8 {
            return Err(TrapCause::StoreAccessFault { address: offset });
        }
        let mut value_bytes = register_value.to_le_bytes();
        let update_range = offset..(offset + bytes.len());
        value_bytes[update_range].copy_from_slice(bytes);
        Ok(u64::from_le_bytes(value_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_write_to_mtime_low_word_updates_mtime() {
        let mut bus = build_bus_state();
        bus.direct_write(MTIME, &0x11223344u32.to_le_bytes()).unwrap();
        assert_eq!(bus.clint.mtime, 0x11223344);
    }

    #[test]
    fn test_direct_write_to_mtime_high_word_updates_mtime() {
        let mut bus = build_bus_state();
        bus.direct_write(MTIME + 4, &0x11223344u32.to_le_bytes()).unwrap();
        assert_eq!(bus.clint.mtime, 0x1122334400000000);
    }

    #[test]
    fn test_direct_read_from_mtime_low_word_returns_low_bits() {
        let mut bus = build_bus_state();
        bus.clint.mtime = 0x1122334455667788;
        assert_eq!(bus.direct_read(MTIME, 4).unwrap(), 0x55667788);
    }

    #[test]
    fn test_direct_read_from_mtime_high_word_returns_high_bits() {
        let mut bus = build_bus_state();
        bus.clint.mtime = 0x1122334455667788;
        assert_eq!(bus.direct_read(MTIME + 4, 4).unwrap(), 0x11223344);
    }

    #[test]
    fn test_direct_write_to_mtimecmp_updates_mtimecmp() {
        let mut bus = build_bus_state();
        bus.direct_write(MTIMECMP, &0xAABBCCDDu32.to_le_bytes()).unwrap();
        assert_eq!(bus.clint.mtimecmp, 0xAABBCCDD);
    }

    #[test]
    fn test_direct_read_from_mtimecmp_returns_low_bits() {
        let mut bus = build_bus_state();
        bus.clint.mtimecmp = 0xAABBCCDD11223344;
        assert_eq!(bus.direct_read(MTIMECMP, 4).unwrap(), 0x11223344);
    }
}
