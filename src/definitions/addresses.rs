// Machine-level CSR addresses. 
// Each is 12 bits, 3 hex digits, 4 bits apiece 
// Bits [11:10] (R/W vs. read-only) and bits [9:8] (required privilege level) 
// sit right next to each other, together spanning bits [11:8] 
// All five CSRs below are read/write + M-only, i.e. 00(RW) + 11(M) = 0011 = 3,
// which is why every one of them starts with 3. 
// 0xF11: 11(read-only) + 11(M) = 1111 = F
// The remaining two digits (bits [7:0]) don't carry a specified per-bit meaning 
// they just give each CSR its own unique slot within its access/privilege class; 
// mstatus bit layout (RV32). One 32-bit CSR, carved into many independent
// named fields at different bit positions -- touching one field (e.g. MPP)
// must not disturb any of the others sharing this same register. Laid out
// left (bit 31) to right (bit 0), same direction as every other diagram in
// this codebase; split across two lines since it doesn't fit in one.
//
// |SD|WPRI |SDT|SPELP|TSR|TW|TVM|MXR|SUM|MPRV|XS   |FS   |**MPP**|VS  |SPP|MPIE|UBE|SPIE|WPRI|MIE|WPRI|SIE|WPRI|
// |31|30:25|24 |23   |22 |21|20 |19 |18 |17  |16:15|14:13|12:11  |10:9|8  |7   |6  |5   |4   |3  |2   |1  |0   |
//
pub const MSTATUS: usize = 0x300;
pub const MTVEC: usize = 0x305;   // the address a trap jumps pc to (BASE field, Direct mode).
pub const MEPC: usize = 0x341;    // the pc of the instruction that trapped, saved for later resume.
pub const MCAUSE: usize = 0x342;  // a code identifying why the last trap happened (see TrapCause::mcause_code).
pub const MTVAL: usize = 0x343;   // extra trap-specific info: a faulting address, or illegal-instruction bits.

pub const CYCLE: usize = 0xC00; // 1100_0000_0000, shadows MCYCLE
pub const TIME: usize = 0xC01;
pub const INSTRET: usize = 0xC02; // shadows MINSTRET

pub const MCYCLE: usize = 0xB00;
pub const MINSTRET: usize = 0xB02; 
pub const WFI_FUNCT_TWELVE:usize = 0x105;