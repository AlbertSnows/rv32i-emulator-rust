#[derive(Debug, PartialEq)]
pub enum ExecutionSignal {
    Continue,
    Halt
}

// RISC-V privilege levels -
// Higher number = more privileged
//   - CPUMode's own numeric encoding representing what mode the CPU is CURRENTLY in.
//   - the value found in a CSR address's bits [9:8] 
//      the minimum privilege level required to access that specific CSR 
pub const PRIV_U: u32 = 0; // User/Application -- least privileged; ordinary program code.
pub const PRIV_S: u32 = 1; // Supervisor -- an OS kernel, on a hart that implements S-mode.
// Level 2 is Reserved -- not used by any of RISC-V's three currently-defined levels.
pub const PRIV_M: u32 = 3; // Machine -- most privileged; the only mode every RISC-V
                            // hart is guaranteed to implement; can access every CSR.

enum AccessStates {
    ReadOnly,
    WriteOnly,
    ReadWrite
}

// misa ("Machine ISA") -- fixed value reported by this emulator, read-only
// (see riscv_privleged.pdf 3.1.1). Bits 31:30 are MXL (native width);
// the rest is one bit per letter of the alphabet, bit N = the Nth letter
// (A=0, B=1, ... I=8 ... M=12 ... S=18 ... U=20 ... Z=25), set for every
// extension/mode this codebase actually implements.
//
//   bits 31:30  MXL = 1        -> RV32 (see Table 11)
//   bit  20     U              -> U-mode implemented (CPUMode::U)
//   bit  18     S              -> S-mode implemented
//   bit  12     M              -> M extension implemented
//   bit  8      I              -> RV32I base ISA
//   bit  0      A              -> A extension implemented
//
// 0x40_14_11_01 = (1 << 30) | (1 << 20) | (1 << 18) | (1 << 12) | (1 << 8) | (1 << 0)
pub const MISA_STATE: u32 = 0x40141101;