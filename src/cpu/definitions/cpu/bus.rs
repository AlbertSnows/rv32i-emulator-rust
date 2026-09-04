use crate::cpu::definitions::addresses::{MSIP, MSIP_END, MSTATUS, MTIME, MTIMECMP, MTIMECMP_END, MTIME_END, PLIC, PLIC_END, UART, UART_END};
use crate::cpu::definitions::cpu::cpu_definition::CPUMode;
use crate::cpu::definitions::cpu::csr::CSRState;
use crate::cpu::definitions::cpu::memory::{MemoryAccessType, MemoryState, build_memory_state};
use crate::cpu::definitions::masks::{MPP, MSTATUS_MPRV};
use crate::cpu::definitions::trap_cause::TrapCause;
use crate::cpu::mmu;
use crate::peripherals::plic::{NUM_CONTEXTS, NUM_SOURCES, PlicState};
use crate::peripherals::uart::{UART_SOURCE_ID, UartState};
use crate::utility::bit_operations::{extract_sub_bytes, mask_and_shift};
use crate::utility::types::as_byte_type;

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
    pub clint: ClintState,
    pub plic: PlicState,
    pub uart: UartState,
}

pub fn build_bus_state() -> BUSState {
    BUSState {
        ram: build_memory_state(),
        clint: ClintState { mtime: 0, mtimecmp: 0, msip: 0 },
        plic: PlicState {
            priority: [0; NUM_SOURCES + 1],
            pending: [false; NUM_SOURCES + 1],
            enabled: [[false; NUM_SOURCES + 1]; NUM_CONTEXTS],
            threshold: [0; NUM_CONTEXTS],
            armed: [true; NUM_SOURCES + 1]
        },
        uart: UartState {
            rx_buffer: std::collections::VecDeque::new()
        }
    }
}

impl BUSState {

    pub fn receive_uart_byte(&mut self, byte: u8) {
        self.uart.receive_byte(byte);
        self.plic.set_pending(UART_SOURCE_ID);
    }

    // For instruction fetch only. Tagged
    // MemoryAccessType::Fetch, so the walker checks the PTE's X
    // (execute) bit -- not R. Never used for data accesses; a load
    // reading through here would be checked against the wrong
    // permission bit.
    pub fn guest_fetch(&mut self,
                       addr: u32,
                       num_bytes: usize,
                       state: &CSRState,
                       mode: CPUMode) -> Result<u32, TrapCause> {
        let phs_addr = if mode == CPUMode::M {
            addr
        } else {
            mmu::lookup_virt_to_phys(
                addr,
                MemoryAccessType::Fetch,
                self,
                state,
                mode
            )?
        };

        self.direct_read(phs_addr as usize, num_bytes)
    }

    // For ordinary data reads a guest instruction performs
    // Tagged MemoryAccessType::Load, so the walker checks the PTE's R
    // (read) bit, not X. Using guest_fetch here instead would check
    // the wrong permission bit: an ordinary data page would incorrectly fail every
    // load against it, while an executable-but-not-readable page would
    // incorrectly let a load through that should have faulted.
    pub fn guest_load(&mut self,
                      addr: u32,
                      num_bytes: usize,
                      state: &CSRState,
                      mode: CPUMode) -> Result<u32, TrapCause> {
        let mstatus = state.read(MSTATUS, CPUMode::M)?;
        let mprv_set = mask_and_shift(mstatus, MSTATUS_MPRV) == 1;
        let mpp_lvl = mask_and_shift(mstatus, MPP);
        let as_priv  = CPUMode::from_privilege_level(mpp_lvl)?;
        let effective_mode = if mprv_set { as_priv } else { mode };

        let phs_addr = if effective_mode == CPUMode::M {
            addr
        } else {
            mmu::lookup_virt_to_phys(
                addr,
                MemoryAccessType::Load,
                self,
                state,
                effective_mode
            )?
        };
        self.direct_read(phs_addr as usize, num_bytes)
    }

    pub fn guest_write(&mut self, addr: u32,
                       bytes: &[u8],
                       state: &CSRState,
                       mode: CPUMode) -> Result<(), TrapCause> {
        let mstatus = state.read(MSTATUS, CPUMode::M)?;
        let mprv_set = mask_and_shift(mstatus, MSTATUS_MPRV) == 1;
        let mpp_lvl = mask_and_shift(mstatus, MPP);
        let as_priv  = CPUMode::from_privilege_level(mpp_lvl)?;
        let effective_mode = if mprv_set { as_priv } else { mode };

        let phs_addr = if effective_mode == CPUMode::M {
            addr
        } else {
            mmu::lookup_virt_to_phys(
                addr,
                MemoryAccessType::Store,
                self,
                state,
                effective_mode
            )?
        };
        self.direct_write(phs_addr as usize, bytes)
    }

    pub fn direct_read(&mut self, address: usize, num_bytes: usize) -> Result<u32, TrapCause> {
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
            MSIP..=MSIP_END => {
                let msip_offset = address - MSIP;
                self.clint.read_msip(msip_offset, num_bytes)
            },
            UART..=UART_END => {
                let offset = address - UART;
                Ok(self.uart.read(offset as u32, num_bytes))
            },
            PLIC..=PLIC_END => {
                let offset = address - PLIC;
                Ok(self.plic.read(offset as u32, num_bytes))
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
            MSIP..=MSIP_END => {
                let msip_offset = address - MSIP;
                self.clint.write_msip(msip_offset, bytes)
            },
            UART..=UART_END => {
                let offset = address - UART;
                Ok(self.uart.write(offset as u32, bytes))
            },
            PLIC..=PLIC_END => {
                let offset = address - PLIC;
                Ok(self.plic.write(offset as u32, bytes))
            },
            _ => {
                if address < BASE_ADDRESS as usize {
                    return Err(TrapCause::StoreAccessFault { address });
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
    pub msip: u32
}

impl ClintState {

    pub fn read_msip(&self, offset: usize, num_bytes: usize) -> Result<u32, TrapCause> {
        Self::read_register(self.msip as u64, offset, num_bytes)
    }

    pub fn write_msip(&mut self, offset: usize, bytes: &[u8]) -> Result<(), TrapCause> {
        self.msip = Self::write_register(self.msip as u64, offset, bytes)? as u32;
        Ok(())
    }

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

    #[test]
    fn test_direct_read_from_uart_lsr_offset_returns_ready() {
        let mut bus = build_bus_state();
        // offset 5 = LSR; 0x60 = THRE|TEMT bits set, "always ready to send"
        assert_eq!(bus.direct_read(UART + 5, 1).unwrap(), 0x60);
    }

    #[test]
    fn test_direct_read_from_uart_other_offset_returns_zero() {
        let mut bus = build_bus_state();
        assert_eq!(bus.direct_read(UART + 1, 1).unwrap(), 0);
    }

    #[test]
    fn test_direct_write_to_uart_thr_offset_succeeds() {
        let mut bus = build_bus_state();
        // offset 0 = THR; writing here is what triggers printing the byte
        assert!(bus.direct_write(UART, &[b'H']).is_ok());
    }

    #[test]
    fn test_direct_write_to_uart_other_offset_succeeds() {
        let mut bus = build_bus_state();
        // any offset other than 0 is currently a no-op, but must not error/panic
        assert!(bus.direct_write(UART + 1, &[0]).is_ok());
    }

    #[test]
    fn test_uart_address_range_is_distinct_from_ram() {
        let mut bus = build_bus_state();
        // a write just past UART_END should fall through to the normal
        // BASE_ADDRESS/RAM path, not be swallowed as a UART access
        assert_eq!(
            bus.direct_write(UART_END + 1, &[0]).unwrap_err(),
            TrapCause::StoreAccessFault { address: UART_END + 1 }
        );
    }
}
