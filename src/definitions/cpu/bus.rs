use crate::definitions::cpu::memory::{MemoryState, build_memory_state};
use crate::definitions::trap_cause::{TrapCause};
use crate::definitions::addresses::{MTIME, MTIMECMP, MTIME_END, MTIMECMP_END};
use crate::utility::types::{ ByteType, as_byte_type };
use crate::utility::bit_operations::{ as_window, extract_sub_bytes };

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
                let offset = address - MTIME;
                let width = as_byte_type(num_bytes).ok_or(TrapCause::LoadAccessFault { address })?;
                let sub_bytes = extract_sub_bytes(self.clint.mtime, offset, width);
                Ok(sub_bytes as u32)
            },
            MTIMECMP..=MTIMECMP_END => {
                let offset = address - MTIMECMP;
                let width = as_byte_type(num_bytes).ok_or(TrapCause::LoadAccessFault { address })?;
                let sub_bytes = extract_sub_bytes(self.clint.mtimecmp, offset, width);
                Ok(sub_bytes as u32)            },
            _ => {
                self.ram.read_bytes(address, num_bytes)
            }
        }
    }
    pub fn direct_write(&mut self, address: usize, bytes: &[u8]) -> Result<(), TrapCause> {
        match address {
            MTIME..=MTIME_END => {
                let offset = address - MTIME;
                let mut mtime_bytes = self.clint.mtime.to_le_bytes();
                let update_range = offset..(offset + bytes.len());
                mtime_bytes[update_range].copy_from_slice(bytes);
                self.clint.mtime = u64::from_le_bytes(mtime_bytes);
                Ok(())
            },
            MTIMECMP..=MTIMECMP_END => {
                let offset = address - MTIMECMP;
                let mut mtimecmp_bytes = self.clint.mtimecmp.to_le_bytes();
                let update_range = offset..(offset + bytes.len());
                mtimecmp_bytes[update_range].copy_from_slice(bytes);
                self.clint.mtimecmp = u64::from_le_bytes(mtimecmp_bytes);
                Ok(())
            },
            _ => {
                self.ram.write_bytes(address, bytes)
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
