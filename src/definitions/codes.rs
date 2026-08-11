#[derive(Debug, PartialEq)]
pub enum ExecutionSignal {
    Continue,
    Halt
}

pub const MTVEC: usize = 0x305;
pub const MEPC: usize = 0x341;
pub const MCAUSE: usize = 0x342;
pub const MTVAL: usize = 0x343;