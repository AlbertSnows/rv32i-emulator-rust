// Machine-level addresses.
// Remember: addresses are just a number
// 0x1234 tells you where to start, it does not hold the value itself
// you need a predefined entity,E, (array) to define a range of bits to hold the value
// E[0x1234] is getting the value at that address

pub mod csr;
pub mod mmio;

pub use csr::*;
pub use mmio::*;

// WFI's funct12 (bits 31:20 of its SYSTEM-opcode encoding), not a CSR
// address -- happens to share STVEC's numeric value (0x105) coincidentally.
pub const WFI_FUNCT_TWELVE: usize = 0x105;
