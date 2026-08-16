use crate::definitions::cpu::cpu_definition::{CPUState, CPUMode};
use crate::definitions::codes::ExecutionSignal;
use crate::definitions::addresses::{MTVEC, MEPC, MCAUSE, MTVAL, MSTATUS, MIE, MIP};
use crate::fetcher::fetch_word_from_memory;
use crate::decoder::decode_word_to_instruction;
use crate::instructions::pc::advance_pc;
use crate::definitions::trap_cause::TrapCause;
use crate::utility::bit_operations::{set_bit_range, mask_and_shift};
use crate::definitions::cpu::cpu_definition::CPUCycles;
use crate::definitions::cpu::csr::MIPBits;
use crate::definitions::masks::{GLOBAL_MIE, MPIE, MTIP, MTIE};

fn perform_step(cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    // mut allows cpu to change in the local scope
    let raw_word = fetch_word_from_memory(&cpu.pc, &cpu.mem)?; // 51 = 0x33 = 0011 0011
    let instruction = decode_word_to_instruction(raw_word)?;
    // &mut cpu passes a mutable reference to cpu
    // &mut cpu = this reference has "mutable" permission to cpu
    let execution_outcome = instruction.execute(cpu).map_err(|err| match err {
        TrapCause::IllegalInstruction { instruction: None } => TrapCause::IllegalInstruction { instruction: Some(raw_word.0) },
        other => other,
    })?;
    advance_pc(&mut cpu.pc, &instruction, &cpu.register)?;
    Ok(execution_outcome)
}

// mepc ("Machine Exception Program Counter") -- MRW, address 0x341.
// "When a trap is taken into M-mode, mepc is written with the virtual
// address of the instruction that was interrupted or that encountered
// the exception."
fn set_mepc(cpu: &mut CPUState) {
    let pc_value = cpu.pc.read();
    cpu.csr.guest_write(MEPC, pc_value as u32, CPUMode::M)
        .expect("MEPC is 0b00_11, mode is M, MEPC is matched");
}

// mcause ("Machine Cause") -- MRW, address 0x342.
// "When a trap is taken into M-mode, mcause is written with a code
// indicating the event that caused the trap."
fn set_mcause(cpu: &mut CPUState, cause_code: u32) {
    cpu.csr.guest_write(MCAUSE, cause_code, CPUMode::M)
        .expect("MCAUSE is 0b00_11, mode is M, MCAUSE is matched");
}

// mtval ("Machine Trap Value") -- MRW, address 0x343.
// "When a trap is taken into M-mode, mtval is either set to zero or
// written with exception-specific information to assist software in
// handling the trap." For address-related exceptions, that's the
// faulting virtual address; for IllegalInstruction, optionally the
// faulting instruction bits; otherwise zero.
fn set_mtval(cpu: &mut CPUState, trap_cause: &TrapCause) {
    let trap_val = match trap_cause {
        TrapCause::InstructionAddressMisaligned { address } |   
        TrapCause::InstructionAccessFault { address } |   
        TrapCause::LoadAddressMisaligned { address } |  
        TrapCause::LoadAccessFault { address } |
        TrapCause::StoreAddressMisaligned { address } |  
        TrapCause::StoreAccessFault { address } => *address as u32,
        TrapCause::IllegalInstruction { instruction } => instruction.unwrap_or(0), 
        TrapCause::Breakpoint | TrapCause::EnvironmentCallFromMMode | 
        TrapCause::EnvironmentCallFromSMode | TrapCause::EnvironmentCallFromUMode |
        TrapCause::MachineTimerInterrupt
            => 0
    };
    cpu.csr.guest_write(MTVAL, trap_val, CPUMode::M)
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
fn jump_to_trap_handler(cpu: &mut CPUState) {
    // MTVEC is a real, always-implemented address -- this read cannot fail.
    cpu.pc.write(cpu.csr.read(MTVEC).expect("MTVEC is matched") as usize);
}

fn set_mpp(cpu: &mut CPUState) {
    let mstatus_state = cpu.csr.read(MSTATUS).expect("MSTATUS is matched");
    let privilege_level = cpu.mode.as_privilege_level();
    let updated_mstatus = set_bit_range(mstatus_state, privilege_level, 2, 11);
    cpu.csr.guest_write(MSTATUS, updated_mstatus, CPUMode::M)
        .expect("MSTATUS is 0b00_11, mode is M, MSTATUS is matched");

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
    if cpu.flags.in_trap {
        return ExecutionSignal::Halt;
    }
    cpu.flags.in_trap = true;
    set_mepc(cpu); // store pc
    set_mpp(cpu); // save the mode to mstatus
    cpu.mode = CPUMode::M;
    set_mcause(cpu, trap_cause.mcause_code()); // store why the trap happened for guest function

    set_mtval(cpu, &trap_cause);
    let mstatus = cpu.csr.read(MSTATUS).expect("MSTATUS is defined");
    let mie = mask_and_shift(mstatus, GLOBAL_MIE);
    let mstatus_after_mie = set_bit_range(mstatus, 0, 1, GLOBAL_MIE.trailing_zeros() as usize);
    let mstatus_after_mpie = set_bit_range(mstatus_after_mie, mie, 1, MPIE.trailing_zeros() as usize);
    cpu.csr.guest_write(MSTATUS, mstatus_after_mpie, CPUMode::M).expect("Writing to MSTATUS is safe.");
    // "If mtval is written with a nonzero value when 
    // a breakpoint, address-misaligned, access-fault, page-fault, or hardware-error exception occurs 
    // on an instruction fetch, load, or store, 
    // then mtval will contain the faulting virtual address.
    jump_to_trap_handler(cpu); // write the trap handler address to pc to go there
    ExecutionSignal::Continue
}

pub fn step(cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    cpu.csr.update_cycle(CPUCycles::Cycle);
    cpu.mem.update_time();
    cpu.csr.update_mip_pending_bit(MIPBits::MTI, (cpu.mem.mtime >= cpu.mem.mtimecmp) as u32);
    let interrupt_detected = mask_and_shift(cpu.csr.read(MSTATUS).expect("MSTATUS defined"), GLOBAL_MIE) == 1 &&
                             mask_and_shift(cpu.csr.read(MIP).expect("MIP defined"), MTIP) == 1 &&
                             mask_and_shift(cpu.csr.read(MIE).expect("MIE defined"), MTIE) == 1;
    if interrupt_detected {
        Ok(handle_trap(cpu, TrapCause::MachineTimerInterrupt))
    } else {
        match perform_step(cpu) {
            Ok(signal) => {
                cpu.csr.update_cycle(CPUCycles::Instret);
                Ok(signal)
            },
            Err(trap_cause) => Ok(handle_trap(cpu, trap_cause))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu::cpu_definition::build_cpu_state;
    use crate::utility::bit_operations::{store_in_mem, mask_and_shift};
    use crate::programs::instructions::ADD_X3_X1_X2;
    use crate::programs::instructions::NO_OP;
    use crate::definitions::masks;
    use crate::definitions::addresses::{CYCLE, INSTRET};

    #[test]
    fn test_step_executes_add_and_advances_pc() {
        let mut cpu = build_cpu_state();
        cpu.register.write(1, 10);
        cpu.register.write(2, 7);
        // to le bytes converts to [0x00, 0x20, 0x81, 0xB3]
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.mem, 0);
        let outcome = step(&mut cpu);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
        assert_eq!(cpu.register.read(3), 17);
        assert_eq!(cpu.pc.read(), 4);
    }

    #[test]
    fn test_step_increments_instret_on_successful_retire() {
        let mut cpu = build_cpu_state();
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.mem, 0);
        step(&mut cpu);
        assert_eq!(cpu.csr.read(CYCLE).unwrap(), 1);
        assert_eq!(cpu.csr.read(INSTRET).unwrap(), 1);
    }

    #[test]
    fn test_step_does_not_increment_instret_on_trap() {
        // an undefined opcode traps -- cycle still "costs" a cycle, but the
        // instruction never retires, so instret must not increment.
        // in other words, instret counts successful steps, and thus should be 0 here
        let mut cpu = build_cpu_state();
        store_in_mem(&NO_OP.to_le_bytes(), &mut cpu.mem, 0);
        step(&mut cpu);
        assert_eq!(cpu.csr.read(CYCLE).unwrap(), 1);
        assert_eq!(cpu.csr.read(INSTRET).unwrap(), 0);
    }

    #[test]
    fn test_step_returns_ok_on_undefined_opcode() {
        let mut cpu = build_cpu_state();
        // 0b0000000 isn't a real opcode -- decode should fail
        store_in_mem(&NO_OP.to_le_bytes(), &mut cpu.mem, 0);
        let outcome = step(&mut cpu);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
    }

    #[test]
    fn test_handle_trap_saves_mode_into_mpp_from_non_m_mode() {
        // this test ensures that csr write has M access in handle trap jumper
        let mut cpu = build_cpu_state();
        cpu.mode = CPUMode::S;
        cpu.pc.write(7);
        cpu.csr.guest_write(MTVEC, 4, CPUMode::M);
        handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33) });
        let mpp = mask_and_shift(cpu.csr.read(MSTATUS).unwrap(), masks::MPP);
        assert_eq!(mpp, CPUMode::S.as_privilege_level());
        assert_eq!(cpu.mode, CPUMode::M);
    }

    #[test]
    fn test_handle_trap_changes_correct_values() {
        let mut cpu = build_cpu_state();
        cpu.pc.write(7);
        cpu.csr.guest_write(MTVEC, 4, CPUMode::M);
        let outcome = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33)});
        assert_eq!(cpu.pc.read(), 4); // check MTVEC
        assert_eq!(cpu.csr.read(MEPC).unwrap(), 7); // mepc
        assert_eq!(cpu.csr.read(MCAUSE).unwrap(), 2); // mcause
        assert_eq!(cpu.csr.read(MTVAL).unwrap(), 33); // mtval
    }

    #[test]
    fn test_handle_trap_sets_in_trap_flag() {
        let mut cpu = build_cpu_state();
        cpu.csr.guest_write(MTVEC, 4, CPUMode::M);
        assert_eq!(cpu.flags.in_trap, false);
        handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33) });
        assert_eq!(cpu.flags.in_trap, true);
    }

    #[test]
    fn test_handle_trap_returns_halt_on_double_trap() {
        let mut cpu = build_cpu_state();
        cpu.csr.guest_write(MTVEC, 4, CPUMode::M);
        // first trap enters the handler normally
        let first = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(1) });
        assert_eq!(first, ExecutionSignal::Continue);
        // a second trap, before MRET clears in_trap, is a double trap
        let second = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(2) });
        assert_eq!(second, ExecutionSignal::Halt);
    }

    #[test]
    fn test_step_takes_interrupt_when_pending_enabled_and_mie_set() {
        // when a timer interrupt is due
        // the cpu takes it instead of running the instruction it was about to execute

        // mtime >= mtimecmp, MTIE=1, mstatus.MIE=1 
        let mut cpu = build_cpu_state();
        cpu.register.write(2, 2);
        cpu.register.write(1, 1);
        // mie = per-bit interrupt
        // mtie = bit 7 of mie, allows machine timer interrupt
        cpu.csr.guest_write(MIE, MTIE, CPUMode::M);
        // mstatus mie = global interrupt enable
        cpu.csr.guest_write(MSTATUS, GLOBAL_MIE, CPUMode::M);
        // mtvec = trap handler location
        cpu.csr.guest_write(MTVEC, 3, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.mem, 1);
        cpu.pc.write(1);
        step(&mut cpu);
        // step() should trap
        // into the handler instead of running the next instruction: 
        // pc jumps to mtvec, mepc captures the pre-interrupt pc, 
        // mcause == 0x8000_0007. 
        // Confirm the instruction that would've run didn't.
        assert_eq!(cpu.pc.read(), 3);
        assert_eq!(cpu.csr.read(MEPC).unwrap(), 1);
        assert_eq!(cpu.csr.read(MCAUSE).unwrap(), 0x8000_0007);
        assert_eq!(cpu.register.read(3), 0);
        


    }

    #[test]
    fn test_step_does_not_interrupt_when_mstatus_mie_clear() {
        // same setup as above but mstatus.MIE=0 -- should run the pending
        // instruction normally instead of trapping.
    }

    #[test]
    fn test_step_does_not_interrupt_when_mtie_clear() {
        // pending + mstatus.MIE=1, but MTIE=0 -- should run normally.
    }

    #[test]
    fn test_step_does_not_interrupt_when_mtip_not_pending() {
        // mtimecmp set higher than mtime, MTIE=1, mstatus.MIE=1 -- MTIP
        // never goes pending, should run normally. Careful: mtime and
        // mtimecmp both default to 0, and 0 >= 0 is true, so mtimecmp
        // needs to be deliberately set above mtime for this one.
    }

    #[test]
    fn test_machine_timer_interrupt_sets_mtval_to_zero() {
        // after the interrupt fires, mtval should read 0 -- no faulting
        // address applies to an interrupt.
    }

    #[test]
    fn test_mret_restores_real_mie_value_captured_on_trap_entry() {
        // mstatus.MIE=1 before any trap; after entry, MPIE should have
        // captured 1 and MIE should be 0; after MRET, MIE should read
        // back as 1 -- proves entry's MPIE=MIE capture and MRET's
        // MIE=MPIE restore round-trip a real value now, not stale data.
    }

    #[test]
    fn test_interrupt_arriving_while_already_in_trap_halts() {
        // cpu.flags.in_trap already true, then conditions for a timer
        // interrupt become true -- should hit the existing double-trap
        // path (Halt), not enter the handler again.
    }
}