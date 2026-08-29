// I-type
//
//  31                  20 19    15 14   12 11     7 6      0
// |      imm[11:0]      |  rs1   | funct3 |   rd    | opcode |
// |        12           |   5    |   3    |    5    |   7    |
//
// rd <- rs1 OP imm[11:0]   (sign-extended, except shift amounts / csr / ecall / ebreak)
// one register operand in, one register operand out, one 12-bit immediate.
// e.g. addi, slti, sltiu, xori, ori, andi, slli, srli, srai, jalr,
//      lb, lh, lw, lbu, lhu, ecall, ebreak, csrrw, csrrs, csrrc, csrrwi, csrrsi, csrrci

pub mod load;
pub mod alu_imm_or_shift;
pub mod shift;
pub mod system;
pub mod jalr;
pub mod csr;

use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use alu_imm_or_shift::IShOp;

