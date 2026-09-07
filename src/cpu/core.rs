use crate::cpu::decoder::decode_word_to_instruction;
use crate::cpu::definitions::addresses;
use crate::cpu::definitions::addresses::{MIE, MIP, MSTATUS, SSTATUS};
use crate::cpu::definitions::codes::ExecutionSignal;
use crate::cpu::definitions::cpu::cpu_definition::{CPUMode, CPUState};
use crate::cpu::definitions::addresses::CsrAddress;
use crate::cpu::definitions::cpu::csr::{CPUCycles, InterruptSource};
use crate::cpu::definitions::masks::{GLOBAL_MIE, GLOBAL_SIE, MEIE, MEIP, MPIE, MTIE, MTIP, SEIE, SEIP, STIE, STIP};
use crate::cpu::definitions::trap_cause::{M_TRAP, S_TRAP, TrapCause};
use crate::cpu::fetcher::fetch_word_from_memory;
use crate::cpu::instructions::i::system::inst_i_xret;
use crate::cpu::instructions::pc::advance_pc;
use crate::cpu::trap::handle_trap;
use crate::peripherals::plic::{M_CONTEXT, S_CONTEXT};
use crate::utility::bit_operations::mask_and_shift;
use crate::utility::types::ByteType;

fn execute_instruction(cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    // mut allows cpu to change in the local scope
    let raw_word = fetch_word_from_memory(&cpu.pc, &mut cpu.bus,  &cpu.csr, cpu.mode)?; // 51 = 0x33 = 0011 0011
    let instruction = decode_word_to_instruction(raw_word)?;
    // &mut cpu passes a mutable reference to cpu
    // &mut cpu = this reference has "mutable" permission to cpu
    let execution_outcome = instruction.execute(cpu).map_err(|err| match err {
        TrapCause::IllegalInstruction { instruction: None } => TrapCause::IllegalInstruction { instruction: Some(raw_word.0) },
        other => other,
    })?;
    let advance_amount =
        if raw_word.1 == ByteType::Word { ByteType::Word.as_num() }
        else { ByteType::HalfWord.as_num() };
    advance_pc(&mut cpu.pc, &instruction, &mut cpu.register, advance_amount as u32)?;
    Ok(execution_outcome)
}

pub fn cycle(cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    cpu.csr.update_cycle(CPUCycles::Cycle);
    cpu.bus.clint.update_time();
    cpu.csr.set_interrupt_pending(InterruptSource::MTI, (cpu.bus.clint.mtime >= cpu.bus.clint.mtimecmp));
    cpu.csr.set_interrupt_pending(InterruptSource::MEI, cpu.bus.plic.compute_eip(M_CONTEXT));
    cpu.csr.set_interrupt_pending(InterruptSource::SEI, cpu.bus.plic.compute_eip(S_CONTEXT));

    if let Some(cause) = select_pending_interrupt(cpu) {
       return Ok(handle_trap(cpu, cause))
    }

    match execute_instruction(cpu) {
        Ok(signal) => {
            if (!cpu.csr.take_and_reset_instret_state()) {
                cpu.csr.update_cycle(CPUCycles::Instret);
            }
            Ok(signal)
        },
        Err(trap_cause) => Ok(handle_trap(cpu, trap_cause))
    }

}

fn select_pending_interrupt(cpu: &CPUState) -> Option<TrapCause> {
    let mip = cpu.csr.read(MIP, CPUMode::M).expect("MIP defined");
    let mie = cpu.csr.read(MIE, CPUMode::M).expect("MIE defined");
    // bit field to get what's pending

    // RISC-V Instruction Set Manual, Privileged Architecture, §3.1.9 "Machine Interrupt (mip and mie) Registers," page 699
    let interrupts_by_priority = [
        (TrapCause::MachineExternalInterrupt, MEIE, MEIP),
        (TrapCause::MachineTimerInterrupt, MTIE, MTIP),
        (TrapCause::SupervisorExternalInterrupt, SEIE, SEIP),
        (TrapCause::SupervisorTimerInterrupt, STIE, STIP)
    ];
    interrupts_by_priority.iter().find_map(|(cause, enabled_mask, pending_mask)| {
        check_interrupt(mip, mie, *pending_mask, *enabled_mask, cpu, *cause)
    })
}

pub fn check_interrupt(mip: u32,
                       mie: u32,
                       pending_mask: u32,
                       enabled_mask: u32,
                       cpu: &CPUState,
                       cause: TrapCause) -> Option<TrapCause> {
    let is_pending = mask_and_shift(mip, pending_mask) == 1;
    // whether or not we're willing to listen
    let is_enabled = mask_and_shift(mie, enabled_mask) == 1;

    // if current_mode < target_mode, cpu cannot block the interrupt, must handle
    let current_level = cpu.mode.as_privilege_level();
    let target_level = TrapCause::target_mode_for(cause).as_privilege_level();
    let (status_field, global_field) = match cpu.mode {
        CPUMode::M => (MSTATUS, GLOBAL_MIE),
        CPUMode::S => (SSTATUS, GLOBAL_SIE),
        CPUMode::U => (CsrAddress::new(0).unwrap(), 0),
    };
    let must_handle_interrupt = current_level < target_level
        || (current_level == target_level
            && mask_and_shift(cpu.csr.read(status_field, CPUMode::M).expect("STATUS defined"), global_field) == 1);
    if is_pending  && is_enabled && must_handle_interrupt {
        return Some(cause);
    }
    None
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::definitions::addresses::{CYCLE, INSTRET};
    use crate::cpu::definitions::cpu::bus::BASE_ADDRESS;
    use crate::cpu::definitions::cpu::cpu_definition::build_cpu_state;
    use crate::cpu::definitions::masks;
    use crate::cpu::programs::instructions::ADD_X3_X1_X2;
    use crate::cpu::programs::instructions::JALR_X1_X1_0;
    use crate::cpu::programs::instructions::NO_OP;
    use crate::utility::bit_operations::{mask_and_shift, store_in_mem};

    #[test]
    fn test_step_executes_add_and_advances_pc() {
        let mut cpu = build_cpu_state();
        cpu.register.write(1, 10);
        cpu.register.write(2, 7);
        // to le bytes converts to [0x00, 0x20, 0x81, 0xB3]
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 0);
        cpu.pc.write(BASE_ADDRESS as usize);
        let outcome = cycle(&mut cpu);
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
        let outcome = cycle(&mut cpu);
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
        cycle(&mut cpu);
        assert_eq!(cpu.csr.read(CYCLE, CPUMode::M).unwrap(), 1);
        assert_eq!(cpu.csr.read(INSTRET, CPUMode::M).unwrap(), 1);
    }

    #[test]
    fn test_step_does_not_increment_instret_on_trap() {
        // an undefined opcode traps -- cycle still "costs" a cycle, but the
        // instruction never retires, so instret must not increment.
        // in other words, instret counts successful steps, and thus should be 0 here
        let mut cpu = build_cpu_state();
        store_in_mem(&NO_OP.to_le_bytes(), &mut cpu.bus.ram, 0);
        cycle(&mut cpu);
        assert_eq!(cpu.csr.read(CYCLE, CPUMode::M).unwrap(), 1);
        assert_eq!(cpu.csr.read(INSTRET, CPUMode::M).unwrap(), 0);
    }

    #[test]
    fn test_step_returns_ok_on_undefined_opcode() {
        let mut cpu = build_cpu_state();
        // 0b0000000 isn't a real opcode -- decode should fail
        store_in_mem(&NO_OP.to_le_bytes(), &mut cpu.bus.ram, 0);
        let outcome = cycle(&mut cpu);
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
    }

    #[test]
    fn test_handle_trap_saves_mode_into_mpp_from_non_m_mode() {
        // this test ensures that csr write has M access in handle trap jumper
        let mut cpu = build_cpu_state();
        cpu.mode = CPUMode::S;
        cpu.pc.write(7);
        cpu.csr.guest_write(addresses::MTVEC, 4, CPUMode::S);
        let outcome = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33) });
        let mpp = mask_and_shift(cpu.csr.read(addresses::MSTATUS, CPUMode::M).unwrap(), masks::MPP);
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
        assert_eq!(cpu.csr.read(addresses::MEPC, CPUMode::M).unwrap(), 7); // mepc
        assert_eq!(cpu.csr.read(addresses::MCAUSE, CPUMode::M).unwrap(), 2); // mcause
        assert_eq!(cpu.csr.read(addresses::MTVAL, CPUMode::M).unwrap(), 33); // mtval
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
        assert_eq!(cpu.csr.read(addresses::MTVAL, CPUMode::M).unwrap(), 2);
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
        cpu.csr.guest_write(addresses::MTVEC, 100, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 1);
        cpu.pc.write(1);
        cycle(&mut cpu);
        // step() should trap
        // into the handler instead of running the next instruction: 
        // pc jumps to mtvec, mepc captures the pre-interrupt pc, 
        // top bit = is interrupt
        // bottob bit tranlates trap cause
        // mcause == 0x8000_0007. 
        // Confirm the instruction that would've run didn't.
        assert_eq!(cpu.pc.read(), 100);
        assert_eq!(cpu.csr.read(addresses::MEPC, CPUMode::M).unwrap(), 1);
        assert_eq!(cpu.csr.read(addresses::MCAUSE, CPUMode::M).unwrap(), 0x8000_0007);
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
        cycle(&mut cpu);
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 8);
        assert_eq!(cpu.csr.read(addresses::MEPC, CPUMode::M).unwrap(), 0);
        assert_eq!(cpu.csr.read(addresses::MCAUSE, CPUMode::M).unwrap(), 0);
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
        cycle(&mut cpu);
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 8);
        assert_eq!(cpu.csr.read(addresses::MEPC, CPUMode::M).unwrap(), 0);
        assert_eq!(cpu.csr.read(addresses::MCAUSE, CPUMode::M).unwrap(), 0);
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
        cycle(&mut cpu);
        assert_eq!(cpu.pc.read(), BASE_ADDRESS as usize + 8);
        assert_eq!(cpu.csr.read(addresses::MEPC, CPUMode::M).unwrap(), 0);
        assert_eq!(cpu.csr.read(addresses::MCAUSE, CPUMode::M).unwrap(), 0);
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
        cpu.csr.guest_write(addresses::MTVEC, 100, CPUMode::M);
        store_in_mem(&ADD_X3_X1_X2.to_le_bytes(), &mut cpu.bus.ram, 4);
        cpu.pc.write(4);
        cpu.csr.guest_write(addresses::MTVAL, 999, CPUMode::M);
        let outcome = cycle(&mut cpu);
        assert_eq!(cpu.pc.read(), 100);
        assert_eq!(cpu.csr.read(addresses::MEPC, CPUMode::M).unwrap(), 4);
        assert_eq!(cpu.csr.read(addresses::MCAUSE, CPUMode::M).unwrap(), 0x8000_0007);
        assert_eq!(cpu.csr.read(addresses::MTVAL, CPUMode::M).unwrap(), 0);
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
        assert_eq!(mask_and_shift(cpu.csr.read(addresses::MSTATUS, CPUMode::M).unwrap(), GLOBAL_MIE), 1);
        handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(0x1234) });
        let mstatus_after_trap = cpu.csr.read(addresses::MSTATUS, CPUMode::M).unwrap();
        assert_eq!(mask_and_shift(mstatus_after_trap, MPIE), 1);
        assert_eq!(mask_and_shift(mstatus_after_trap, GLOBAL_MIE), 0);
        inst_i_xret(&mut cpu, &M_TRAP);
        assert_eq!(mask_and_shift(cpu.csr.read(addresses::MSTATUS, CPUMode::M).unwrap(), GLOBAL_MIE), 1);
    }
}