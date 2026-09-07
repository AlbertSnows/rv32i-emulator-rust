use crate::cpu::definitions::addresses;
use crate::cpu::definitions::codes::ExecutionSignal;
use crate::cpu::definitions::cpu::cpu_definition::{CPUMode, CPUState};
use crate::cpu::definitions::masks::{GLOBAL_MIE, GLOBAL_SIE, MEIP, MPIE, MTIP, SEIP, SPIE, STIP};
use crate::cpu::definitions::trap_cause::{M_TRAP, S_TRAP};
use crate::cpu::definitions::trap_cause::{TrapCause, TrapDestination};
use crate::utility::bit_operations::{mask_and_shift, set_bit_range};

// mepc ("Machine Exception Program Counter") -- MRW, address 0x341.
// "When a trap is taken into M-mode, mepc is written with the virtual
// address of the instruction that was interrupted or that encountered
// the exception."
fn set_epc(cpu: &mut CPUState, dest: &TrapDestination) {
    let pc_value = cpu.pc.read();
    cpu.csr.guest_write(dest.epc, pc_value as u32, dest.mode)
        .expect("MEPC is 0b00_11, mode is M, MEPC is matched");
}

// mcause ("Machine Cause") -- MRW, address 0x342.
// "When a trap is taken into M-mode, mcause is written with a code
// indicating the event that caused the trap."
fn set_cause(cpu: &mut CPUState, dest: &TrapDestination, cause_code: u32) {
    cpu.csr.guest_write(dest.cause, cause_code, dest.mode)
        .expect("MCAUSE is 0b00_11, mode is M, MCAUSE is matched");
}

// mtval ("Machine Trap Value") -- MRW, address 0x343.
// "When a trap is taken into M-mode, mtval is either set to zero or
// written with exception-specific information to assist software in
// handling the trap." For address-related exceptions, that's the
// faulting virtual address; for IllegalInstruction, optionally the
// faulting instruction bits; otherwise zero.
fn set_tval(cpu: &mut CPUState, dest: &TrapDestination, trap_cause: &TrapCause) {
    let trap_val = match trap_cause {
        TrapCause::InstructionAddressMisaligned { address } |
        TrapCause::InstructionAccessFault { address } |
        TrapCause::LoadAddressMisaligned { address } |
        TrapCause::LoadAccessFault { address } |
        TrapCause::StoreAddressMisaligned { address } |
        TrapCause::StoreAccessFault { address } |
        TrapCause::InstructionPageFault { address } |
        TrapCause::LoadPageFault { address } |
        TrapCause::StorePageFault { address }
            => *address as u32,
        TrapCause::IllegalInstruction { instruction } => instruction.unwrap_or(0),
        TrapCause::Breakpoint | TrapCause::EnvironmentCallFromMMode |
        TrapCause::EnvironmentCallFromSMode | TrapCause::EnvironmentCallFromUMode |
        TrapCause::MachineTimerInterrupt |
        TrapCause::MachineExternalInterrupt |
        TrapCause::SupervisorExternalInterrupt |
        TrapCause::SupervisorTimerInterrupt
            => 0,
    };
    cpu.csr.guest_write(dest.tval, trap_val, dest.mode)
        .expect("MTVAL is 0b00_11, mode is M, MTVAL is matched");
}

// mtvec ("Machine Trap-Vector Base-Address") -- MRW, address 0x305.
// "The mtvec register... holds trap vector configuration, consisting of
// a vector base address (BASE) and a vector mode (MODE)." In Direct mode
// (MODE=0), "all traps set pc to BASE" -- that's the only mode this
// emulator needs to handle right now (no interrupts implemented yet, so
// Vectored mode's per-cause offsets have nothing to apply to).
// docs/research/riscv_privleged.pdf, 3.1.7 "Machine Trap-Vector
// Base-Address (mtvec) Register", p.41.
fn jump_to_trap_handler(cpu: &mut CPUState, dest: &TrapDestination) {
    // MTVEC is a real, always-implemented address -- this read cannot fail.
    cpu.pc.write(cpu.csr.read(dest.tvec, cpu.mode).expect("MTVEC is matched") as usize);
}

// previous privilege, mstatus is deliberately not part of TrapDestination and never varies by
// dest: it's the one physical register regardless of whether this trap
// enters M or S. sstatus is only a masked *view* of this same storage,
// used by guest CSR instructions (csrrw sstatus, ...) -- this function is
// the CPU's own internal bookkeeping, not a guest CSR access, so it
// always reads/writes the real register directly via MSTATUS. What
// varies per-destination is only *which bits* of mstatus get touched --
// that's what dest.pp_mask (below) and dest.ie_mask/pie_mask (in
// set_pie) capture.
fn set_pp(cpu: &mut CPUState, dest: &TrapDestination) {
    let mstatus_state = cpu.csr.read(addresses::MSTATUS, CPUMode::M).expect("MSTATUS is matched");
    let privilege_level = cpu.mode.as_privilege_level();
    let width = dest.pp_mask.count_ones() as usize;
    let position = dest.pp_mask.trailing_zeros() as usize;
    let updated_mstatus = set_bit_range(mstatus_state, privilege_level, width, position);
    cpu.csr.guest_write(addresses::MSTATUS, updated_mstatus, CPUMode::M)
        .expect("MSTATUS is 0b00_11, mode is M, MSTATUS is matched");

}

// "When a trap is taken from privilege mode y into privilege mode x,
// xPIE is set to the value of xIE; xIE is set to 0." -- captures the
// current global interrupt-enable (MIE) into MPIE, then clears MIE, so
// no further interrupts fire while this trap handler is running. MRET's
// existing restore step (MIE=MPIE, MPIE=1) is the mirror image of this.
fn set_pie(cpu: &mut CPUState, dest: &TrapDestination) {
    let mstatus = cpu.csr.read(addresses::MSTATUS, CPUMode::M).expect("MSTATUS is defined");
    let mie = mask_and_shift(mstatus, dest.ie_mask);
    let global_ie = if dest.mode == CPUMode::S { GLOBAL_SIE } else { GLOBAL_MIE };
    let pie = if dest.mode == CPUMode::S { SPIE } else { MPIE };
    let mstatus_after_mie = set_bit_range(mstatus, 0, 1, global_ie.trailing_zeros() as usize);
    let mstatus_after_mpie = set_bit_range(mstatus_after_mie, mie, 1, pie.trailing_zeros() as usize);
    cpu.csr.guest_write(addresses::MSTATUS, mstatus_after_mpie, CPUMode::M).expect("Writing to MSTATUS is safe.");
}

// Here we want to transfer control.
// Where do we store our address
// exceprt:
// When a trap is taken into M-mode, mepc is written with the virtual address of
/// the instruction that was interrupted or that encountered the exception.
// Where do we fetch the address for the trap handler?
// mtvec
// exceprt:
// When MODE=Direct,
/// all traps into machine mode cause the pc to be set to the address in the BASE field.
// When MODE=Vectored, all synchronous exceptions into machine mode cause the pc to be
/// set to the address in the BASE field, whereas interrupts cause the pc to be set to
/// the address in the BASE field plus four times the interrupt cause number.
// For example, a machine-mode timer interrupt (see Table 16) causes the pc to be set to BASE+0x1c.
// This function sets everything up for our guest function to handle the trap.
// - where we came from
// - where to go
// - why the trap was proc'd
// - the address of the instruction that caused the failure
pub fn handle_trap(cpu: &mut CPUState, trap_cause: TrapCause) -> ExecutionSignal {
    cpu.flags.in_trap = true;
    let trapping_mode = cpu.mode;
    // to delegate refers to whether we handle it at the m or s/u level
    let mode_can_delegate = matches!(trapping_mode, CPUMode::S | CPUMode::U);
    // the legs are used to indicate a trap wishes to be handled in a different mode
    // typically signaled by the os. interrupt => mideleg, exceptions => medeleg
    // both are bit fields. each bit represents a different trap code
    let register_value = match trap_cause {
        TrapCause::MachineTimerInterrupt |
        TrapCause::MachineExternalInterrupt |
        TrapCause::SupervisorExternalInterrupt |
        TrapCause::SupervisorTimerInterrupt
            => cpu.csr.read(addresses::MIDELEG, CPUMode::M),
        _ => cpu.csr.read(addresses::MEDELEG, CPUMode::M),
    }.expect("mideleg and medeleg are defined");
    // the mcause code is the location in its corresponding leg
    // in the case of interrupts, the 31st bit is the tag bit, to distinguish them, so we need to
    // strip the tag bit
    let corresponding_mask = match trap_cause {
        TrapCause::MachineTimerInterrupt => MTIP,
        TrapCause::MachineExternalInterrupt => MEIP,
        TrapCause::SupervisorExternalInterrupt => SEIP,
        TrapCause::SupervisorTimerInterrupt => STIP,
        _ => 1 << trap_cause.mcause_code(),
    };
    let relevant_bit_set = mask_and_shift(register_value, corresponding_mask) == 1;
    let is_s_mode = mode_can_delegate && relevant_bit_set;
    let dest = if is_s_mode { &S_TRAP } else { &M_TRAP };
    set_epc(cpu, dest); // store pc
    set_pp(cpu, dest); // save the mode to mstatus
    cpu.mode = dest.mode;
    set_cause(cpu, dest, trap_cause.mcause_code()); // store why the trap happened for guest function
    set_tval(cpu, dest, &trap_cause); // set info about where it failed/which address
    set_pie(cpu, dest); // capture MIE into MPIE, then clear MIE
    // "If mtval is written with a nonzero value when
    // a breakpoint, address-misaligned, access-fault, page-fault, or hardware-error exception occurs
    // on an instruction fetch, load, or store,
    // then mtval will contain the faulting virtual address.
    jump_to_trap_handler(cpu, dest); // write the trap handler address to pc to go there
    ExecutionSignal::Continue
}
