pub mod r;
pub mod b;
pub mod i;
pub mod j;
pub mod s;
pub mod u;
use r::AluOp;
use crate::definitions::cpu_definition::CPUState;

#[derive(Debug, PartialEq)]
pub enum Format {
    RType { op: AluOp, rd: usize, rs1: usize, rs2: usize },
    JType { op: JOp, rd: usize, rs1: usize, rs2: usize },
    UType { op: UOp, rd: usize, rs1: usize, rs2: usize },
    IType { op: IOp, rd: usize, rs1: usize, imm: usize },
    IShiftType { op: IShOp, rd: usize, rs1: usize, shamt: usize },
    SType { op: SOp, rd: usize, rs1: usize, rs2: usize },
    BType { op: BOp, rd: usize, rs1: usize, rs2: usize },
}

impl Format {
    pub fn execute(&self, cpu_state: &mut CPUState) {
        match self {
            Format::RType { op, rd, rs1, rs2 } // all &references because self is &self
                => r::execute_r_type(op, *rd, *rs1, *rs2, &mut cpu_state.register),
            Format::IType { op, rd, rs1, imm } 
                => i::execute_i_type(op, *rd, *rs1, *imm, &mut cpu_state.register),
            Format::IShiftType { op, rd, rs1, shamt } 
                => i::execute_i_shift_type(op, *rd, *rs1, *shamt, &mut cpu_state.register),
            _ => panic!("Unrecognized instruction type")
        }
    }
}