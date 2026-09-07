// MMIO = memory mapped input output

use crate::utility::types::ByteType;

// mtime/mtimecmp are memory-mapped
// registers. read/written via ordinary loads/stores like any
// other address. Owned by CLINT ("Core-Local Interruptor").
// mtime is a free-running counter; mtimecmp is
// the compare value. once mtime >= mtimecmp, a machine timer
// interrupt becomes pending. Both are 64-bit
// despite this being an RV32 (32-bit) emulator --
// a 32-bit CPU accesses them via two separate 32-bit loads/stores (low
// word, then high word)
pub const MTIME: usize = 0x0200BFF8;
pub const MTIMECMP: usize = 0x02004000;
pub const MTIME_END: usize = MTIME + ByteType::DoubleWord.as_num() - 1;
pub const MTIMECMP_END: usize = MTIMECMP + ByteType::DoubleWord.as_num() - 1;

// uart is the console
pub const UART: usize = 0x10000000;
// include up to but not including the 0x100
pub const UART_END: usize = UART + 0x100 - 1;

pub const PLIC: usize = 0x0c00_0000;
pub const PLIC_END: usize = PLIC + 0x201007; // covers both contexts' claim registers
pub const MSIP: usize = 0x0200_0000;
pub const MSIP_END: usize = MSIP + ByteType::DoubleWord.as_num() - 1;
