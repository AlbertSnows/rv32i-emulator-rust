use crate::definitions::cpu_definition::RegisterFile;
use crate::fetcher::InstructionWord;
use crate::instructions::Format;
use crate::definitions::codes::ExecutionSignal;
use crate::utility::bit_operations::mask_and_shift;
use crate::definitions::masks;
use crate::utility::bit_operations::shake_to_signed;

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

pub fn parse_alu_imm_or_shift_inst(raw_word: InstructionWord) -> Result<Format, String> {
    let content = raw_word.0;
    let funct_three = mask_and_shift(content, masks::FUNCT_THREE);
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    match funct_three {
        (0b000 | 0b010 | 0b011 | 0b100 | 0b110 | 0b111) => parse_i_alu_imm(&content, &funct_three, reg_dest, reg_source_one),
        (0b001 | 0b101) => parse_i_shift(&content, &funct_three, reg_dest, reg_source_one),
        _ => Err(format!("Unrecognized funct three format"))
    }
}

pub fn parse_i_shift(content: &u32, funct_three: &u32, reg_dest: u32, reg_source_one: u32) -> Result<Format, String> {
    let shamt = mask_and_shift(*content, masks::I_TYPE_SHAMT);
    let funct_seven = mask_and_shift(*content, masks::FUNCT_SEVEN);
    let instruction_name = match (funct_seven, funct_three) {
        (0b0000000, 0b001) => Ok(IShOp::Slli),
        (0b0000000, 0b101) => Ok(IShOp::Srli),
        (0b0100000, 0b101) => Ok(IShOp::Srai),
        _ => Err(format!("undefined shift type type detected"))
    }?;
    Ok(Format::IShiftType {
        op: instruction_name,
        shamt: shamt as usize,
        rd: reg_dest as usize,
        rs1: reg_source_one as usize
    })
}

pub fn parse_i_alu_imm(content: &u32, funct_three: &u32, reg_dest: u32, reg_source_one: u32) -> Result<Format, String> {
    let imm_unsigned = mask_and_shift(*content, masks::I_TYPE_ALU_IMM);
    let imm_val = shake_to_signed(imm_unsigned, 12);
    let instruction_name = match funct_three {
        0b000 => Ok(AluImmOp::Addi),
        0b010 => Ok(AluImmOp::Slti),
        0b011 => Ok(AluImmOp::Sltiu),
        0b100 => Ok(AluImmOp::Xori),
        0b110 => Ok(AluImmOp::Ori),
        0b111 => Ok(AluImmOp::Andi),
        _ => Err(format!("undefined alu imm type detected"))
    }?;
    Ok(Format::AluImmType {
        op: instruction_name,
        imm: imm_val,
        rd: reg_dest as usize,
        rs1: reg_source_one as usize
    })
}

pub fn execute_i_alu_imm_type(op: &AluImmOp, rd: usize, rs1: usize, imm: i32, register: &mut RegisterFile) -> Result<ExecutionSignal, String> {
    match op {
        AluImmOp::Addi => inst_i_addi(rd, rs1, imm, register),
        AluImmOp::Slti => inst_i_slti(rd, rs1, imm, register),
        AluImmOp::Sltiu => inst_i_sltiu(rd, rs1, imm, register),
        AluImmOp::Xori => inst_i_xori(rd, rs1, imm, register),
        AluImmOp::Ori => inst_i_ori(rd, rs1, imm, register),
        AluImmOp::Andi => inst_i_andi(rd, rs1, imm, register),
    }
    Ok(ExecutionSignal::Continue)
}

pub fn inst_i_addi(rd: usize, rs1: usize, imm_i: i32, reg_file: &mut RegisterFile) {
    // rd <- rs1 + imm_i
    let val = reg_file.read(rs1);
    let imm_u = imm_i as u32;
    let outcome = val.wrapping_add(imm_u);
    reg_file.write(rd, outcome);
}

pub fn inst_i_slti(rd: usize, rs1: usize, imm_i: i32, reg_file: &mut RegisterFile) {
    // rd <- (rs1 <s imm_i) ? 1 : 0
    let val = reg_file.read(rs1) as i32;
    let outcome = if val < imm_i { 1 } else { 0 };
    reg_file.write(rd, outcome);
}

pub fn inst_i_sltiu(rd: usize, rs1: usize, imm_i: i32, reg_file: &mut RegisterFile) {
    // rd <- (rs1 <u imm_i) ? 1 : 0
    let val = reg_file.read(rs1);
    let imm_u = imm_i as u32;
    let outcome = if val < imm_u { 1 } else { 0 };
    reg_file.write(rd, outcome);
}

pub fn inst_i_xori(rd: usize, rs1: usize, imm_i: i32, reg_file: &mut RegisterFile) {
    // rd <- rs1 ^ imm_i
    let val = reg_file.read(rs1);
    let imm_u = imm_i as u32;
    let outcome = val ^ imm_u;
    reg_file.write(rd, outcome);
}

pub fn inst_i_ori(rd: usize, rs1: usize, imm_i: i32, reg_file: &mut RegisterFile) {
    // rd <- rs1 | imm_i
    let val = reg_file.read(rs1);
    let imm_u = imm_i as u32;
    let outcome = val | imm_u;
    reg_file.write(rd, outcome);
}

pub fn inst_i_andi(rd: usize, rs1: usize, imm_i: i32, reg_file: &mut RegisterFile) {
    // rd <- rs1 & imm_i
    let val = reg_file.read(rs1);
    let imm_u = imm_i as u32;
    let outcome = val & imm_u;
    reg_file.write(rd, outcome);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_definition::build_register_file;

    #[test]
    fn test_parse_alu_imm_or_shift_inst_routes_to_alu_imm() {
        // addi x5, x1, 10 -- funct3 = 000, routes to parse_i_alu_imm
        let raw_word = InstructionWord(0x00A08293);
        let result = parse_alu_imm_or_shift_inst(raw_word);
        assert_eq!(result, Ok(Format::AluImmType { op: AluImmOp::Addi, rd: 5, rs1: 1, imm: 10 }));
    }

    #[test]
    fn test_parse_alu_imm_or_shift_inst_routes_to_shift() {
        // slli x1, x2, 3 -- funct3 = 001, funct7 = 0000000, routes to parse_i_shift
        let raw_word = InstructionWord(0x00311093);
        let result = parse_alu_imm_or_shift_inst(raw_word);
        assert_eq!(result, Ok(Format::IShiftType { op: IShOp::Slli, rd: 1, rs1: 2, shamt: 3 }));
    }

    #[test]
    fn test_inst_i_addi() {
        let mut reg = build_register_file();
        let rd = 1;
        let rs1 = 2;
        let imm_i = 3;
        reg.write(2, 2);
        inst_i_addi(rd, rs1, imm_i, &mut reg);
        assert_eq!(reg.read(rd), 5);
    }

    #[test]
    fn test_inst_i_slti() {
        let mut reg = build_register_file();
        let rd = 1;
        let rs1 = 5;
        let imm_i = -8;
        reg.write(5, 2);
        inst_i_slti(rd, rs1, imm_i, &mut reg);
        assert_eq!(reg.read(rd), 0);
    }

    #[test]
    fn test_inst_i_sltiu() {
        let mut reg = build_register_file();
        let rd = 1;
        let rs1 = 5;
        let imm_i = 8;
        reg.write(5, 2);
        inst_i_sltiu(rd, rs1, imm_i, &mut reg);
        assert_eq!(reg.read(rd), 1);
    }

    #[test]
    fn test_inst_i_xori() {
        let mut reg = build_register_file();
        let rd = 1;
        let rs1 = 3;
        let imm_i = 4;
        reg.write(3, 2);
        inst_i_xori(rd, rs1, imm_i, &mut reg);
        assert_eq!(reg.read(rd), 6);
    }

    #[test]
    fn test_inst_i_ori() {
        let mut reg = build_register_file();
        let rd = 1;
        let rs1 = 10;
        let imm_i = 0b1100;
        reg.write(10, 0b1010);
        inst_i_ori(rd, rs1, imm_i, &mut reg);
        assert_eq!(reg.read(rd), 0b1110);
    }

    #[test]
    fn test_inst_i_andi() {
        let mut reg = build_register_file();
        let rd = 1;
        let rs1 = 3;
        let imm_i = 0b0011;
        reg.write(3, 0b0001);
        inst_i_andi(rd, rs1, imm_i, &mut reg);
        assert_eq!(reg.read(rd), 0b0001);
    }
}