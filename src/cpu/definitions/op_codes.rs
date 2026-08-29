// Named opcode constants — one per distinct RV32I opcode value (bits 6:0).
// Several instructions can share the same opcode (see the reference card),
// so there are 10 constants here even though there are only 6 formats.
// Fill in each value from the RV32I Reference Card encoding table.
// the bits are typed as u32, so they're padded out with 0's in the front

// These instructions have to do with loading/fetching from mem via a register by indexing via imm
pub const LOAD: u32 = 0b0000011; // I-type: lb, lh, lw, lbu, lhu
// similar to R type. these ones take one register value, rs1, and one constant imm. 
// all these commands do some operation with rs1 and imm and put the result in rd
// note: three instructions use shamt instead of imm, which is a narrower field of 5 bits (0-31) to do shifting with.
// these are slli, srli, srai. 
pub const ALU_IMM: u32 = 0b0010011; // I-type: addi, slti, sltiu, xori, ori, andi, slli, srli, srai

// These interface with the system teh program is running on, not the program (cpu)'s data
pub const SYSTEM: u32 = 0b1110011; // I-type: ecall, ebreak, csrrw, csrrs, csrrc, csrrwi, csrrsi, csrrci

pub const R: u32 = 0b0110011; // R-type: add, sub, and, or, xor, sll, srl, sra, slt, sltu
pub const JALR: u32 = 0b1100111; // I-type: jalr
pub const S: u32 = 0b0100011; // S-type: sb, sh, sw
pub const B: u32 = 0b1100011; // B-type: beq, bne, blt, bge, bltu, bgeu
pub const LUI: u32 = 0b0110111; // U-type: lui
pub const AUIPC: u32 = 0b0010111; // U-type: auipc
pub const J: u32 = 0b1101111; // J-type: jal
pub const MISC_MEM: u32 = 0b000_1111;
pub const FENCE: u32 = MISC_MEM;

pub const A: u32 = 0b0101111;
