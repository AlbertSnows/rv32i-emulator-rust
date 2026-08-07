pub mod r;
pub mod b;
pub mod i;
pub mod j;
pub mod s;
pub mod u;
use r::AluOp;

#[derive(Debug)]
pub enum Instruction {
    RType { op: AluOp, rd: usize, rs1: usize, rs2: usize },
    JType,
    UType,
    IType,
    SType,
    BType
}