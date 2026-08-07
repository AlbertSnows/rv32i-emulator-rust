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
 pub fn execute_i_type() {

 }

 pub fn execute_i_shift_type() {

 }

 pub fn parse_i_inst(raw_word: InstructionWord) -> Instruction {
    let content = raw_word.0;
    let reg_dest = mask_and_shift(content, masks::REG_DESTINATION);
    let funct_three = mask_and_shift(content, masks::FUNCT_3);
    let reg_source_one = mask_and_shift(content, masks::REG_SOURCE_ONE);
    let reg_source_two = mask_and_shift(content, masks::REG_SOURCE_TWO);
    let funct_seven = mask_and_shift(content, masks::FUNCT_7);
    let instruction_name = match (funct_seven, funct_three) {
        (0b0000000, 0b000) => AluOp::Add,
        (0b0100000, 0b000) => AluOp::Sub,
        (0b0000000, 0b001) => AluOp::Sll,
        (0b0000000, 0b010) => AluOp::Slt,
        (0b0000000, 0b011) => AluOp::Sltu,
        (0b0000000, 0b100) => AluOp::Xor,
        (0b0000000, 0b101) => AluOp::Srl,
        (0b0100000, 0b101) => AluOp::Sra,
        (0b0000000, 0b110) => AluOp::Or,
        (0b0000000, 0b111) => AluOp::And,
        _ => panic!("undefined R type")
    };
    Instruction::RType { 
        op: instruction_name, 
        rd: reg_dest as usize, 
        rs1: reg_source_one as usize, 
        rs2: reg_source_two as usize 
    }
}