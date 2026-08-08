#[derive(Debug)]
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

#[derive(Debug)]
pub struct MemoryState {
    pub storage: [u8; TEST_MEM_SIZE]
}

const TEST_MEM_SIZE: usize = 4096;
const FULL_MEM_SIZE: usize = 65536;
pub fn build_memory_state() -> MemoryState {
    MemoryState { storage: [0; TEST_MEM_SIZE] }
}

#[derive(Debug)]
pub struct PCState {
    value: usize
}
impl PCState {
    pub fn write(&mut self, new_count: usize) -> usize {
        self.value = new_count;
        self.value
    }
    pub fn read(&self) -> usize {
        self.value
    }
}


pub fn build_pc_state() -> PCState {
    PCState { value: 0 }
}

#[derive(Debug)]
pub struct RegisterFile {
    storage: [u32; 32]
}

impl RegisterFile {
    pub fn write(&mut self, index: usize, value: u32) -> u32 {
        if index != 0 {
            self.storage[index] = value
        }
        self.storage[index]
    }

    pub fn read(&self, index: usize) -> u32 {
        self.storage[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_to_x0_is_ignored() {
        let mut rf = build_register_file();
        let returned = rf.write(0, 42);
        assert_eq!(rf.read(0), 0);
        assert_eq!(returned, 0);
    }

    #[test]
    fn test_write_to_other_register_succeeds() {
        let mut rf = build_register_file();
        let returned = rf.write(5, 42);
        assert_eq!(rf.read(5), 42);
        assert_eq!(returned, 42);
    }
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

