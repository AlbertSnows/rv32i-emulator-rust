pub mod r;
pub mod b;
pub mod i;
pub mod j;
pub mod s;
pub mod u;
pub mod pc;
use r::AluOp;
use crate::definitions::cpu_definition::CPUState;
use u::UOp;
use j::JOp;
use b::BOp;
use s::SOp;
use i::load::LoadOp;
use i::alu_imm::AluImmOp;
use i::shift::IShOp;
use i::system::SystemOp;
use crate::definitions::codes::ExecutionSignal;

#[derive(Debug, PartialEq)]
pub enum Format {
    UType { op: UOp, rd: usize, imm: i32 },
    JType { op: JOp, rd: usize, imm: i32 },
    BType { op: BOp, imm: i32, rs1: usize, rs2: usize },
    SType { op: SOp, imm: i32, rs1: usize, rs2: usize },
    RType { op: AluOp, rd: usize, rs1: usize, rs2: usize },
    LoadType { op: LoadOp, rd: usize, rs1: usize, imm: i32 },
    AluImmType { op: AluImmOp, rd: usize, rs1: usize, imm: i32 },
    JalrType { rd: usize, rs1: usize, imm: i32 },
    IShiftType { op: IShOp, rd: usize, rs1: usize, shamt: usize },
    SystemType { op: SystemOp }
}

impl Format {
    pub fn execute(&self, cpu_state: &mut CPUState) -> Result<ExecutionSignal, String> {
        match self {
            Format::UType { op, rd, imm } 
                => u::execute_u_type(op, *rd, *imm, &mut cpu_state.register),
            Format::JType { op, rd, imm } 
                => j::execute_j_type(op, *rd, *imm, &mut cpu_state.register),
            Format::BType { op, imm, rs1, rs2 } 
                => b::execute_b_type(op, *imm, *rs1, *rs2, &mut cpu_state.register),
            Format::SType { op, imm, rs1, rs2 } 
                => s::execute_s_type(op, *imm, *rs1, *rs2, &mut cpu_state.register),
            Format::RType { op, rd, rs1, rs2 } // all &references because self is &self
                => r::execute_r_type(op, *rd, *rs1, *rs2, &mut cpu_state.register),
            Format::LoadType { op, rd, rs1, imm } 
                => i::load::execute_i_load_type(op, *rd, *rs1, *imm, &mut cpu_state.register),
            Format::AluImmType { op, rd, rs1, imm } 
                => i::alu_imm::execute_i_alu_imm_type(op, *rd, *rs1, *imm, &mut cpu_state.register),
            Format::JalrType { rd, rs1, imm } 
                => i::jalr::execute_i_jalr_type(*rd, *rs1, *imm, &mut cpu_state.register),
            Format::IShiftType { op, rd, rs1, shamt } 
                => i::shift::execute_i_shift_type(op, *rd, *rs1, *shamt, &mut cpu_state.register),
            Format::SystemType { op }
                => i::system::execute_i_system_type(op)
        }
    }
}