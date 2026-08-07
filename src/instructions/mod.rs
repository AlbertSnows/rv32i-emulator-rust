pub mod r;
pub mod b;
pub mod i;
pub mod j;
pub mod s;
pub mod u;
use r::AluOp;
use crate::definitions::cpu_definition::CPUState;
use u::UOp;
use j::JOp;
use b::BOp;
use s::SOp;
use i::LoadOp;
use i::AluImmOp;
use i::IShOp;
use i::SystemOp;

#[derive(Debug, PartialEq)]
pub enum Format {
    UType { op: UOp, rd: usize, imm: usize },
    JType { op: JOp, rd: usize, imm: usize },
    BType { op: BOp, imm: usize, rs1: usize, rs2: usize },
    SType { op: SOp, imm: usize, rs1: usize, rs2: usize },
    RType { op: AluOp, rd: usize, rs1: usize, rs2: usize },
    LoadType { op: LoadOp, rd: usize, rs1: usize, imm: usize },
    AluImmType { op: AluImmOp, rd: usize, rs1: usize, imm: usize },
    JalrType { rd: usize, rs1: usize, imm: usize },
    IShiftType { op: IShOp, rd: usize, rs1: usize, shamt: usize },
    SystemType { op: SystemOp }
}

impl Format {
    pub fn execute(&self, cpu_state: &mut CPUState) {
        match self {
            Format::RType { op, rd, rs1, rs2 } // all &references because self is &self
                => r::execute_r_type(op, *rd, *rs1, *rs2, &mut cpu_state.register),
            // Format::IType { op, rd, rs1, imm } 
            //     => i::execute_i_type(op, *rd, *rs1, *imm, &mut cpu_state.register),
            Format::IShiftType { op, rd, rs1, shamt } 
                => i::execute_i_shift_type(op, *rd, *rs1, *shamt, &mut cpu_state.register),
            Format::SystemType { op }
                => i::execute_i_system_type(op, &mut cpu_state.register),
            _ => panic!("Unrecognized instruction type")
        }
    }
}