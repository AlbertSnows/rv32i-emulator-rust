pub fn advance(pc: &mut PCState, instruction: &Format, reg_file: &RegisterFile) -> usize {
    let pc_value = pc.read() as i32;
    let new_value = match instruction {
        Format::JType { op, rd, imm } => pc_value + *imm,
        Format::JalrType { rd, rs1, imm } => {
            let rs1_val = reg_file.read(*rs1);
            let new_value = (rs1_val as i32 + *imm) & !1;
            new_value
        },
        Format::BType { op, imm, rs1, rs2 } => {
            let rs1_val = reg_file.read(*rs1);
            let rs2_val = reg_file.read(*rs2);
            let imm_val = *imm;
            match op {
                BOp::Beq => if rs1_val == rs2_val { pc_value + imm_val } else { pc_value + 4 },
                BOp::Bne => if rs1_val != rs2_val { pc_value + imm_val } else { pc_value + 4 },
                BOp::Bltu => if rs1_val < rs2_val { pc_value + imm_val } else { pc_value + 4 },
                BOp::Bgeu => if rs1_val >= rs2_val { pc_value + imm_val } else { pc_value + 4 },
                BOp::Blt => if (rs1_val as i32) < (rs2_val as i32) { pc_value + imm_val } else { pc_value + 4 },
                BOp::Bge => if (rs1_val as i32) >= (rs2_val as i32) { pc_value + imm_val } else { pc_value + 4 }
            }
        },
        _ => pc_value + 4
    };
    pc.write(new_value as usize);
    pc.read()
}