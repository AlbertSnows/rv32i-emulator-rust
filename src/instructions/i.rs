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
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::definitions::cpu_definition::RegisterFile;

#[derive(Debug, PartialEq)]
pub enum LoadOp {
    Lb,
    Lh,
    Lw,
    Lbu,
    Lhu
}

#[derive(Debug, PartialEq)]
pub enum AluImmOp {
    Addi,
    Slti,
    Sltiu,
    Xori,
    Ori,
    Andi
}

#[derive(Debug, PartialEq)]
pub enum IShOp {
    Slli, 
    Srli, 
    Srai
}

#[derive(Debug, PartialEq)]
pub enum SystemOp {
    ECall,
    EBreak
}

#[derive(Debug, PartialEq)]
pub enum CsrType {
    Csrrw,
    Csrrs,
    Csrrc,
    Csrrwi,
    Csrrsi,
    Csrrci
}

pub fn execute_i_shift_type(op: &IShOp, rd: usize, rs1: usize, shamt: usize, register: &mut RegisterFile) {

}

 pub fn parse_i_inst(raw_word: InstructionWord) -> Format {
    // let content = raw_word.0;
    // let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    // let funct_three = mask_and_shift(content, masks::FUNCT_3);
    // let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    // let reg_source_two = mask_and_shift(content, masks::REG_SOURCE_TWO);
    // let funct_seven = mask_and_shift(content, masks::FUNCT_7);
    // let instruction_name = match (funct_seven, funct_three) {
    //     (0b0000000, 0b000) => AluOp::Add,

    //     _ => panic!("undefined R type")
    // };
    // Format::RType { 
    //     op: instruction_name, 
    //     rd: reg_dest as usize, 
    //     rs1: reg_source_one as usize, 
    //     rs2: reg_source_two as usize 
    // }
    Format::IShiftType { 
        op: IShOp::Slli,
        rs1: 1,
        rd: 1,
        shamt: 1

    }
}

pub fn execute_i_system_type(op: &SystemOp, register: &mut RegisterFile) {

}