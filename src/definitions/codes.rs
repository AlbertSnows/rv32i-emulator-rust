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
