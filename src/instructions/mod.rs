pub mod r;
pub mod b;
pub mod i;
pub mod j;
pub mod s;
pub mod u;
use r::AluOp;
use crate::definitions::cpu_definition::CPUState;
#[derive(Debug, PartialEq)]
pub enum Instruction {
    RType { op: AluOp, rd: usize, rs1: usize, rs2: usize },
    JType,
    UType,
    IType,
    SType,
    BType
}

impl Instruction {
    pub fn execute(&self, cpu_state: &mut CPUState) {
        match self {
            Instruction::RType { op, rd, rs1, rs2 } // all &references because self is &self
                => r::execute_r_type(op, *rd, *rs1, *rs2, &mut cpu_state.register),
            
            _ => panic!("Unrecognized instruction type")
        }
    }
}