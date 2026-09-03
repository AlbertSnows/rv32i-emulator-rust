// Named instruction-word constants, reused across tests instead of
// repeating the same raw hex/byte literals in every file.

// add x3, x1, x2 -- opcode format=R, funct7=0000000, funct3=000, rd=3, rs1=1, rs2=2
// - opcode (bits 6-0) = 0110011 = R-type opcode ✓
// - rd (bits 11-7) = 00011 = 3 → x3
// - funct3 (bits 14-12) = 000
// - rs1 (bits 19-15) = 00001 = 1 → x1
// - rs2 (bits 24-20) = 00010 = 2 → x2
// - funct7 (bits 31-25) = 0000000
//
// full 32 bits, grouped by field (funct7|rs2|rs1|funct3|rd|opcode):
// 0000000 00010 00001 000 00011 0110011
// same 32 bits, grouped by hex nibble instead (= 0x002081B3):
// 0000 0000 0010 0000 1000 0001 1011 0011
pub const ADD_X3_X1_X2: u32 = 0x002081B3;
pub const NO_OP: u32 = 0x00000000;

// jalr x1, x1, 0 -- opcode format=I(JALR), funct3=000, rd=1, rs1=1, imm=0
// - opcode (bits 6-0)   = 1100111 = JALR ✓
// - rd     (bits 11-7)  = 00001 = 1 → x1
// - funct3 (bits 14-12) = 000
// - rs1    (bits 19-15) = 00001 = 1 → x1
// - imm    (bits 31-20) = 000000000000
//
// rd == rs1: this is the aliasing case where execute() must not clobber
// rs1's value before advance_pc() reads it to compute the jump target.
//
// full 32 bits, grouped by field (imm|rs1|funct3|rd|opcode):
// 000000000000 00001 000 00001 1100111
// same 32 bits, grouped by hex nibble instead (= 0x000080E7):
// 0000 0000 0000 0000 1000 0000 1110 0111
pub const JALR_X1_X1_0: u32 = 0x000080E7;
