pub struct CPUState {
    pub register: RegisterFile,
    pub pc: PCState,
    pub mem: MemoryState
}

pub fn build_cpu_state() -> CPUState {
    CPUState {
        register: build_register_file(),
        pc: build_PC_state(),
        mem: build_memory_state()
    }
}

pub struct MemoryState {
    storage: [u8; 32]
}

fn build_memory_state() -> MemoryState {
    MemoryState { storage: [0; 32] }
}

pub struct PCState {
    pub value: u32
}

fn build_PC_state() -> PCState {
    PCState { value: 0 }
}

pub struct RegisterFile {
    pub storage: [u32; 32]
}

// A file, historically, is an ordered row or collection of things. 
// Think of, "rank and file" -> |||||, an ordered line
// So a register file is an ordered list of storage cells.
// Register file can also be thought of as "all of the CPU's registers grouped together"
fn build_register_file() -> RegisterFile {
 RegisterFile { storage: [
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0,
    0
]
}
}

