use crate::definitions::trap_cause::TrapCause;
use crate::definitions::codes::{ PRIV_M, PRIV_S, PRIV_U };
use crate::definitions::addresses::{ CYCLE, TIME, INSTRET };
use crate::definitions::cpu::csr::{CSRState, build_csr_state};
use crate::definitions::cpu::flags::CPUFlags;
use crate::definitions::cpu::bus::{BUSState, build_bus_state};

#[derive(Debug, PartialEq, Clone)]
pub struct CPUState {
    pub register: RegisterFile,
    pub pc: PCState,
    pub bus: BUSState,
    pub csr: CSRState,
    pub mode: CPUMode,
    pub flags: CPUFlags,
    pub reservation_address: Option<u32>
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CPUMode {
    S,
    M,
    U
}

impl CPUMode {
    pub fn as_privilege_level(&self) -> u32 {
        match self {
            CPUMode::U => PRIV_U,
            CPUMode::S => PRIV_S,
            CPUMode::M => PRIV_M
        }
    }
    pub fn from_privilege_level(lvl: u32) -> Result<CPUMode, TrapCause> {
        match lvl {
            PRIV_U => Ok(CPUMode::U),
            PRIV_S => Ok(CPUMode::S),
            PRIV_M => Ok(CPUMode::M),
            _ => Err(TrapCause::IllegalInstruction { instruction: None })
        }
    }
}

pub fn build_cpu_state() -> CPUState {
    CPUState {
        register: build_register_file(),
        pc: build_pc_state(),
        bus: build_bus_state(),
        csr: build_csr_state(),
        mode: CPUMode::M,
        flags: CPUFlags {
            in_trap: false
        },
        reservation_address: None
    }
}

// Program Counter
#[derive(Debug, Copy, PartialEq, Clone)]
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

#[derive(Debug, Copy, PartialEq, Clone)]
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

}
