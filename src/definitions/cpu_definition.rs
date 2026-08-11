use crate::definitions::trap_cause::TrapCause;

#[derive(Debug)]
pub struct CPUState {
    pub register: RegisterFile,
    pub pc: PCState,
    pub mem: MemoryState,
    pub csr: CsrState
}

pub fn build_cpu_state() -> CPUState {
    CPUState {
        register: build_register_file(),
        pc: build_pc_state(),
        mem: build_memory_state(),
        csr: build_csr_state()
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

impl MemoryState {
    // Writes the low `num_bytes` bytes of `value` to memory starting at
    // `address`, little-endian (least-significant byte at the lowest address)
    // same ordering fetch_word_from_memory reads back.
    // It does not worry about how many bytes to write, that's the responsibility
    // of the parent
    pub fn write_bytes(&mut self, address: usize, bytes: &[u8]) -> Result<(), TrapCause> {
        if address + bytes.len() > self.storage.len() {
            return Err(TrapCause::StoreAccessFault { address });
        }
        bytes.iter().enumerate().for_each(|(i, byte)| {
            self.storage[address + i] = *byte;
        });
        Ok(())
    }

    // Reads `num_bytes` bytes starting at `address` and combines them
    // little-endian (lowest address = least-significant byte) into a u32,
    // the mirror image of write_bytes.
    pub fn read_bytes(&self, address: usize, num_bytes: usize) -> Result<u32, TrapCause> {
        if address + num_bytes > self.storage.len() {
            return Err(TrapCause::LoadAccessFault { address });
        }
        let byte_range = 0..num_bytes;
        let value = byte_range.fold(0u32, |acc, i| {
            let byte_at_index = self.storage[address + i] as u32;
            let positioned_byte = byte_at_index << (i * 8);
            acc | positioned_byte
        });
        Ok(value)
    }
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

// CSR (Control and Status Register) address space is 12 bits wide (0..4096), per the Zicsr extension 
// separate storage from the general-purpose, not reg  file
#[derive(Debug)]
pub struct CsrState {
    storage: [u32; 4096]
}

impl CsrState {
    pub fn write(&mut self, address: usize, value: u32) -> Result<u32, TrapCause> {
        // "The top two bits (csr[11:10]) indicate whether the register is read/write (00, 01, or 10) or read-only (11). 
        // The next two bits (csr[9:8]) encode the lowest privilege level that can access the CSR."
        // https://docs.riscv.org/reference/isa/_attachments/riscv-privileged.pdf
        let valid_write = (address >> 10) & 0b11 != 0b11;
        if valid_write {
            self.storage[address] = value;
            return Ok(self.storage[address]);
        }
        return Err(TrapCause::IllegalInstruction { instruction: None });        
    }

    pub fn read(&self, address: usize) -> u32 {
        self.storage[address]
    }
}

pub fn build_csr_state() -> CsrState {
    CsrState { storage: [0; 4096] }
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

    #[test]
    fn test_pc_write_and_read() {
        let mut pc = build_pc_state();
        assert_eq!(pc.read(), 0);
        let returned = pc.write(4);
        assert_eq!(returned, 4);
        assert_eq!(pc.read(), 4);
    }

    #[test]
    fn test_write_bytes_single_byte() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0b0101_1010;
        mem.write_bytes(8, &test_value.to_le_bytes()).unwrap();
        assert_eq!(mem.storage[8], 0b0101_1010);
    }

    #[test]
    fn test_write_bytes_halfword_little_endian() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0b1111_0000_1010_0101;
        mem.write_bytes(8, &test_value.to_le_bytes()).unwrap();
        assert_eq!(mem.storage[8], 0b1010_0101);
        assert_eq!(mem.storage[9], 0b1111_0000);
    }

    #[test]
    fn test_write_bytes_word_little_endian() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0x12345678;
        mem.write_bytes(8, &test_value.to_le_bytes()).unwrap();
        assert_eq!(mem.storage[8], 0x78);
        assert_eq!(mem.storage[9], 0x56);
        assert_eq!(mem.storage[10], 0x34);
        assert_eq!(mem.storage[11], 0x12);
    }

    #[test]
    fn test_write_bytes_out_of_bounds_returns_err() {
        let mut mem = build_memory_state();
        let test_value: u32 = 0x12345678;
        let outcome = mem.write_bytes(TEST_MEM_SIZE - 2, &test_value.to_le_bytes());
        assert!(outcome.is_err());
    }

    #[test]
    fn test_read_bytes_single_byte() {
        let mut mem = build_memory_state();
        mem.storage[8] = 0b0101_1010;
        assert_eq!(mem.read_bytes(8, 1).unwrap(), 0b0101_1010);
    }

    #[test]
    fn test_read_bytes_halfword_little_endian() {
        let mut mem = build_memory_state();
        mem.storage[8] = 0b1010_0101;
        mem.storage[9] = 0b1111_0000;
        assert_eq!(mem.read_bytes(8, 2).unwrap(), 0b1111_0000_1010_0101);
    }

    #[test]
    fn test_read_bytes_word_little_endian() {
        let mut mem = build_memory_state();
        mem.storage[8] = 0x78;
        mem.storage[9] = 0x56;
        mem.storage[10] = 0x34;
        mem.storage[11] = 0x12;
        assert_eq!(mem.read_bytes(8, 4).unwrap(), 0x12345678);
    }

    #[test]
    fn test_read_bytes_out_of_bounds_returns_err() {
        let mem = build_memory_state();
        let outcome = mem.read_bytes(TEST_MEM_SIZE - 2, 4);
        assert!(outcome.is_err());
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

