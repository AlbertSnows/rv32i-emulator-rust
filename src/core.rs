use crate::definitions::cpu::cpu_definition::{CPUState, CPUMode};
use crate::definitions::codes::ExecutionSignal;
use crate::definitions::addresses;
use crate::fetcher::fetch_word_from_memory;
use crate::decoder::decode_word_to_instruction;
use crate::instructions::pc::advance_pc;
use crate::definitions::trap_cause::{TrapCause, TrapDestination};
use crate::utility::bit_operations::{set_bit_range, mask_and_shift};
use crate::definitions::cpu::csr::{CPUCycles, MIPBits};
use crate::definitions::masks::{GLOBAL_MIE, MPIE, MTIP, MTIE, MTI, MPP, SPP, GLOBAL_SIE, SPIE};
use crate::instructions::i::system::{inst_i_xret};
use crate::definitions::trap_cause::{M_TRAP, S_TRAP};

fn perform_step(cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    // mut allows cpu to change in the local scope
    let raw_word = fetch_word_from_memory(&cpu.pc, &cpu.bus)?; // 51 = 0x33 = 0011 0011
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
        TrapCause::StoreAccessFault { address } => *address as u32,
        TrapCause::IllegalInstruction { instruction } => instruction.unwrap_or(0),
        TrapCause::Breakpoint | TrapCause::EnvironmentCallFromMMode |
        TrapCause::EnvironmentCallFromSMode | TrapCause::EnvironmentCallFromUMode |
        TrapCause::MachineTimerInterrupt
            => 0
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
    cpu.pc.write(cpu.csr.read(dest.tvec).expect("MTVEC is matched") as usize);
}

// mstatus is deliberately not part of TrapDestination and never varies by
// dest: it's the one physical register regardless of whether this trap
// enters M or S. sstatus is only a masked *view* of this same storage,
// used by guest CSR instructions (csrrw sstatus, ...) -- this function is
// the CPU's own internal bookkeeping, not a guest CSR access, so it
// always reads/writes the real register directly via MSTATUS. What
// varies per-destination is only *which bits* of mstatus get touched --
// that's what dest.pp_mask (below) and dest.ie_mask/pie_mask (in
// set_pie) capture.
fn set_pp(cpu: &mut CPUState, dest: &TrapDestination) {
    let mstatus_state = cpu.csr.read(addresses::MSTATUS).expect("MSTATUS is matched");
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
    let mstatus = cpu.csr.read(addresses::MSTATUS).expect("MSTATUS is defined");
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
        TrapCause::MachineTimerInterrupt => cpu.csr.read(addresses::MIDELEG),
        _ => cpu.csr.read(addresses::MEDELEG),
    }.expect("mideleg and medeleg are defined");
    // the mcause code is the location in its corresponding leg
    // in the case of interrupts, the 31st bit is the tag bit, to distinguish them, so we need to
    // strip the tag bit
    let corresponding_mask = match trap_cause {
        TrapCause::MachineTimerInterrupt => MTI,
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

pub fn step(cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    cpu.csr.update_cycle(CPUCycles::Cycle);
    cpu.bus.clint.update_time();
    cpu.csr.update_mip_pending_bit(MIPBits::MTI, (cpu.bus.clint.mtime >= cpu.bus.clint.mtimecmp) as u32);
    let interrupt_detected = mask_and_shift(cpu.csr.read(addresses::MSTATUS).expect("MSTATUS defined"), GLOBAL_MIE) == 1 &&
                             mask_and_shift(cpu.csr.read(addresses::MIP).expect("MIP defined"), MTIP) == 1 &&
                             mask_and_shift(cpu.csr.read(addresses::MIE).expect("MIE defined"), MTIE) == 1;
    if interrupt_detected && !cpu.flags.in_trap {
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
    use crate::definitions::cpu::bus::BASE_ADDRESS;
    use crate::utility::bit_operations::{store_in_mem, mask_and_shift};
    use crate::programs::instructions::ADD_X3_X1_X2;
    use crate::programs::instructions::NO_OP;
    use crate::programs::instructions::JALR_X1_X1_0;
    use crate::definitions::masks;
    use crate::definitions::addresses::{CYCLE, INSTRET};

    #[test]
    fn test_step_executes_add_and_advances_pc() {
        let mut cpu = build_cpu_state();
        cpu.register.write(1, 10);
        cpu.register.write(2, 7);
        // to le bytes converts to [0x00, 0x20, 0x81, 0xB3]
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 0);
        cpu.pc.write(BASE_ADDRESS as usize);
        let outcome = step(&mut cpu);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
        assert_eq!(cpu.register.read(3), 17);
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 4);
    }

    #[test]
    fn test_step_jalr_reads_rs1_before_execute_clobbers_it_when_rd_equals_rs1() {
        // jalr x1, x1, 0 
        // rd and rs1 are the same register. execute()
        // writes rd = pc+4 (the link address); advance_pc() separately
        // reads rs1 to compute the jump target. If execute() runs first,
        // by the time advance_pc() reads rs1 it's already been overwritten
        // with the link address, and the cpu jumps to the wrong place.
        let mut cpu = build_cpu_state();
        cpu.register.write(1, BASE_ADDRESS as usize as u32 + 40);
        store_in_mem(&JALR_X1_X1_0.to_le_bytes(), &mut cpu.bus.ram, 0);
        cpu.pc.write(BASE_ADDRESS as usize);
        let outcome = step(&mut cpu);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
        // jumped to rs1's original value, not the just-written link address
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 40);
        // link address still landed correctly in rd
        assert_eq!(cpu.register.read(1), BASE_ADDRESS + 4);
    }

    #[test]
    fn test_step_increments_instret_on_successful_retire() {
        let mut cpu = build_cpu_state();
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 0);
        cpu.pc.write(BASE_ADDRESS as usize);
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
        store_in_mem(&NO_OP.to_le_bytes(), &mut cpu.bus.ram, 0);
        step(&mut cpu);
        assert_eq!(cpu.csr.read(CYCLE).unwrap(), 1);
        assert_eq!(cpu.csr.read(INSTRET).unwrap(), 0);
    }

    #[test]
    fn test_step_returns_ok_on_undefined_opcode() {
        let mut cpu = build_cpu_state();
        // 0b0000000 isn't a real opcode -- decode should fail
        store_in_mem(&NO_OP.to_le_bytes(), &mut cpu.bus.ram, 0);
        let outcome = step(&mut cpu);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
    }

    #[test]
    fn test_handle_trap_saves_mode_into_mpp_from_non_m_mode() {
        // this test ensures that csr write has M access in handle trap jumper
        let mut cpu = build_cpu_state();
        cpu.mode = CPUMode::S;
        cpu.pc.write(7);
        cpu.csr.guest_write(addresses::MTVEC, 4, CPUMode::M);
        handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33) });
        let mpp = mask_and_shift(cpu.csr.read(addresses::MSTATUS).unwrap(), masks::MPP);
        assert_eq!(mpp, CPUMode::S.as_privilege_level());
        assert_eq!(cpu.mode, CPUMode::M);
    }

    #[test]
    fn test_handle_trap_changes_correct_values() {
        let mut cpu = build_cpu_state();
        cpu.pc.write(7);
        cpu.csr.guest_write(addresses::MTVEC, 4, CPUMode::M);
        let outcome = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33)});
        assert_eq!(cpu.pc.read(), 4); // check MTVEC
        assert_eq!(cpu.csr.read(addresses::MEPC).unwrap(), 7); // mepc
        assert_eq!(cpu.csr.read(addresses::MCAUSE).unwrap(), 2); // mcause
        assert_eq!(cpu.csr.read(addresses::MTVAL).unwrap(), 33); // mtval
    }

    #[test]
    fn test_handle_trap_sets_in_trap_flag() {
        let mut cpu = build_cpu_state();
        cpu.csr.guest_write(addresses::MTVEC, 4, CPUMode::M);
        assert_eq!(cpu.flags.in_trap, false);
        handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33) });
        assert_eq!(cpu.flags.in_trap, true);
    }

    #[test]
    fn test_handle_trap_allows_nested_synchronous_traps() {
        // Real hardware allows synchronous exceptions to nest freely 
        // mepc/mcause/mtval get overwritten by whichever trap fires
        // most recently. Preserving earlier trap state, if software cares,
        // is software's own responsibility (e.g. saving to a stack before
        // doing anything risky), not something the CPU blocks on. 
        // riscv-tests boot code relies on exactly this: it deliberately
        // traps to probe for optional CSRs, points mtvec at the very next
        // instruction, and never runs MRET in between probes.
        let mut cpu = build_cpu_state();
        cpu.csr.guest_write(addresses::MTVEC, 4, CPUMode::M);
        let first = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(1) });
        assert_eq!(first, ExecutionSignal::Continue);
        let second = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(2) });
        assert_eq!(second, ExecutionSignal::Continue);
        // the second trap's info overwrote the first's
        assert_eq!(cpu.csr.read(addresses::MTVAL).unwrap(), 2);
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
        cpu.csr.guest_write(addresses::MIE, MTIE, CPUMode::M);
        // mstatus mie = global interrupt enable
        cpu.csr.guest_write(addresses::MSTATUS, GLOBAL_MIE, CPUMode::M);
        // mtvec = trap handler location
        cpu.csr.guest_write(addresses::MTVEC, 3, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 1);
        cpu.pc.write(1);
        step(&mut cpu);
        // step() should trap
        // into the handler instead of running the next instruction: 
        // pc jumps to mtvec, mepc captures the pre-interrupt pc, 
        // top bit = is interrupt
        // bottob bit tranlates trap cause
        // mcause == 0x8000_0007. 
        // Confirm the instruction that would've run didn't.
        assert_eq!(cpu.pc.read(), 3);
        assert_eq!(cpu.csr.read(addresses::MEPC).unwrap(), 1);
        assert_eq!(cpu.csr.read(addresses::MCAUSE).unwrap(), 0x8000_0007);
        assert_eq!(cpu.register.read(3), 0);
    }

    #[test]
    fn test_step_does_not_interrupt_when_mstatus_mie_clear() {
        // same setup as above but mstatus.MIE=0 -- should run the pending
        // instruction normally instead of trapping.
        let mut cpu = build_cpu_state();
        cpu.register.write(2, 3);
        cpu.register.write(1, 4);
        // mie = per-bit interrupt
        // mtie = bit 7 of mie, allows machine timer interrupt if set
        cpu.csr.guest_write(addresses::MIE, MTIE, CPUMode::M);
        cpu.csr.guest_write(addresses::MSTATUS, 0, CPUMode::M);
        cpu.csr.guest_write(addresses::MTVEC, 3, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 4);
        cpu.pc.write(BASE_ADDRESS as usize + 4);
        step(&mut cpu);
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 8);
        assert_eq!(cpu.csr.read(addresses::MEPC).unwrap(), 0);
        assert_eq!(cpu.csr.read(addresses::MCAUSE).unwrap(), 0);
        assert_eq!(cpu.register.read(3), 7);
    }

    #[test]
    fn test_step_does_not_interrupt_when_mtie_clear() {
        // pending + mstatus.MIE=1, but MTIE=0 -- should run normally.
        let mut cpu = build_cpu_state();
        cpu.register.write(2, 4);
        cpu.register.write(1, 3);
        cpu.csr.guest_write(addresses::MIE, 0, CPUMode::M);
        cpu.csr.guest_write(addresses::MSTATUS, GLOBAL_MIE, CPUMode::M);
        cpu.csr.guest_write(addresses::MTVEC, 3, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 4);
        cpu.pc.write(BASE_ADDRESS as usize + 4);
        step(&mut cpu);
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 8);
        assert_eq!(cpu.csr.read(addresses::MEPC).unwrap(), 0);
        assert_eq!(cpu.csr.read(addresses::MCAUSE).unwrap(), 0);
        assert_eq!(cpu.register.read(3), 7);
    }

    #[test]
    fn test_step_does_not_interrupt_when_mtip_not_pending() {
        // mtimecmp set higher than mtime, MTIE=1, mstatus.MIE=1 -- MTIP
        // never goes pending, should run normally. Careful: mtime and
        // mtimecmp both default to 0, and 0 >= 0 is true, so mtimecmp
        // needs to be deliberately set above mtime for this one.
        let mut cpu = build_cpu_state();
        cpu.register.write(2, 4);
        cpu.register.write(1, 3);
        cpu.csr.guest_write(addresses::MIE, MTIE, CPUMode::M);
        cpu.csr.guest_write(addresses::MSTATUS, GLOBAL_MIE, CPUMode::M);
        cpu.csr.guest_write(addresses::MTVEC, 3, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 4);
        cpu.pc.write(BASE_ADDRESS as usize + 4);
        cpu.bus.clint.mtime = 1;
        cpu.bus.clint.mtimecmp = 4;
        step(&mut cpu);
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 8);
        assert_eq!(cpu.csr.read(addresses::MEPC).unwrap(), 0);
        assert_eq!(cpu.csr.read(addresses::MCAUSE).unwrap(), 0);
        assert_eq!(cpu.register.read(3), 7);
    }

    #[test]
    fn test_machine_timer_interrupt_sets_mtval_to_zero() {
        // after the interrupt fires, mtval should read 0 -- no faulting
        // address applies to an interrupt.
        let mut cpu = build_cpu_state();
        cpu.register.write(2, 4);
        cpu.register.write(1, 3);
        cpu.csr.guest_write(addresses::MIE, MTIE, CPUMode::M);
        cpu.csr.guest_write(addresses::MSTATUS, GLOBAL_MIE, CPUMode::M);
        cpu.csr.guest_write(addresses::MTVEC, 3, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 4);
        cpu.pc.write(4);
        cpu.csr.guest_write(addresses::MTVAL, 999, CPUMode::M);
        step(&mut cpu);
        assert_eq!(cpu.pc.read(), 3);
        assert_eq!(cpu.csr.read(addresses::MEPC).unwrap(), 4);
        assert_eq!(cpu.csr.read(addresses::MCAUSE).unwrap(), 0x8000_0007);
        assert_eq!(cpu.csr.read(addresses::MTVAL).unwrap(), 0);
        assert_eq!(cpu.register.read(3), 0);
    }

    #[test]
    fn test_mret_restores_real_mie_value_captured_on_trap_entry() {
        // mstatus.MIE=1 before any trap; after entry, MPIE should have
        // captured 1 and MIE should be 0; after MRET, MIE should read
        // back as 1 -- proves entry's MPIE=MIE capture and MRET's
        // MIE=MPIE restore round-trip a real value now, not stale data.
        let mut cpu = build_cpu_state();
        cpu.csr.guest_write(addresses::MSTATUS, GLOBAL_MIE, CPUMode::M);
        assert_eq!(mask_and_shift(cpu.csr.read(addresses::MSTATUS).unwrap(), GLOBAL_MIE), 1);
        handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(0x1234) });
        let mstatus_after_trap = cpu.csr.read(addresses::MSTATUS).unwrap();
        assert_eq!(mask_and_shift(mstatus_after_trap, MPIE), 1);
        assert_eq!(mask_and_shift(mstatus_after_trap, GLOBAL_MIE), 0);
        inst_i_xret(&mut cpu, &M_TRAP);
        assert_eq!(mask_and_shift(cpu.csr.read(addresses::MSTATUS).unwrap(), GLOBAL_MIE), 1);
    }

    #[test]
    fn test_step_defers_interrupt_while_already_in_trap() {
        // cpu.flags.in_trap already true, then conditions for a timer
        // interrupt become true 
        // step() must not re-fire the interrupt
        // (that would clobber mepc/mcause and starve whatever handler is
        // already running of the chance to execute even one instruction).
        // It should run the next instruction normally, same as if no
        // interrupt were pending at all. The interrupt stays pending and
        // will correctly fire once in_trap goes back to false via MRET.
        let mut cpu = build_cpu_state();
        cpu.flags.in_trap = true;
        cpu.csr.guest_write(addresses::MIE, MTIE, CPUMode::M);
        cpu.csr.guest_write(addresses::MSTATUS, GLOBAL_MIE, CPUMode::M);
        cpu.register.write(1, 4);
        cpu.register.write(2, 3);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 4);
        cpu.pc.write(BASE_ADDRESS as usize + 4);
        let outcome = step(&mut cpu);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
        assert_eq!(cpu.register.read(3), 7); 
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 8); // advanced normally, not jumped to mtvec
    }
}