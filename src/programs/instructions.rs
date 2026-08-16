// Named instruction-word constants, reused across tests instead of
// repeating the same raw hex/byte literals in every file.

// add x3, x1, x2 -- opcode format=R, funct7=0000000, funct3=000, rd=3, rs1=1, rs2=2
// - opcode (bits 6-0) = 0110011 = R-type opcode ✓
// - rd (bits 11-7) = 00011 = 3 → x3
// - funct3 (bits 14-12) = 000
// - rs1 (bits 19-15) = 00001 = 1 → x1
// - rs2 (bits 24-20) = 00010 = 2 → x2
// - funct7 (bits 31-25) = 0000000
pub const ADD_X3_X1_X2: u32 = 0x002081B3;
pub const NO_OP: u32 = 0x00000000;