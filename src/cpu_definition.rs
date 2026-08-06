pub struct CPUState {
    pub register: RegisterFile,
    pub pc: PCState,
    pub mem: MemoryState
}

pub fn build_cpu_state() -> CPUState {
    CPUState {
        register: build_register_file(),
        pc: build_pc_state(),
        mem: build_memory_state()
    }
}

pub struct MemoryState {
    pub storage: [u8; TEST_MEM_SIZE]
}

const TEST_MEM_SIZE: usize = 4096;
const FULL_MEM_SIZE: usize = 65536;
pub fn build_memory_state() -> MemoryState {
    MemoryState { storage: [0; TEST_MEM_SIZE] }
}

pub struct PCState {
    pub value: usize
}

pub fn build_pc_state() -> PCState {
    PCState { value: 0 }
}

pub struct RegisterFile {
    pub storage: [u32; 32]
}

// A file, historically, is an ordered row or collection of things. 
// Think of, "rank and file" -> |||||, an ordered line
// So a register file is an ordered list of storage cells.
// Register file can also be thought of as "all of the CPU's registers grouped together"
pub fn build_register_file() -> RegisterFile {
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

