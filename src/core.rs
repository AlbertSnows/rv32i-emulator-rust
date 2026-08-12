use crate::definitions::cpu_definition::{CPUState, CPUMode};
use crate::definitions::codes::{ExecutionSignal, MTVEC, MEPC, MCAUSE, MTVAL, MSTATUS};
use crate::fetcher::fetch_word_from_memory;
use crate::decoder::decode_word_to_instruction;
use crate::instructions::pc::advance_pc;
use crate::definitions::trap_cause::TrapCause;
use crate::utility::bit_operations::set_bit_range;

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
fn set_mepc(cpu: &mut CPUState) -> Result<(), TrapCause> {
    let pc_value = cpu.pc.read();
    cpu.csr.write(MEPC, pc_value as u32, CPUMode::M)?;
    Ok(())
}

// mcause ("Machine Cause") -- MRW, address 0x342.
// "When a trap is taken into M-mode, mcause is written with a code
// indicating the event that caused the trap."
fn set_mcause(cpu: &mut CPUState, cause_code: u32) -> Result<(), TrapCause> {
    cpu.csr.write(MCAUSE, cause_code, CPUMode::M)?;
    Ok(())
}

// mtval ("Machine Trap Value") -- MRW, address 0x343.
// "When a trap is taken into M-mode, mtval is either set to zero or
// written with exception-specific information to assist software in
// handling the trap." For address-related exceptions, that's the
// faulting virtual address; for IllegalInstruction, optionally the
// faulting instruction bits; otherwise zero.
fn set_mtval(cpu: &mut CPUState, trap_cause: &TrapCause) -> Result<(), TrapCause> {
    let trap_val = match trap_cause {
        TrapCause::InstructionAddressMisaligned { address } |   
        TrapCause::InstructionAccessFault { address } |   
        TrapCause::LoadAddressMisaligned { address } |  
        TrapCause::LoadAccessFault { address } |
        TrapCause::StoreAddressMisaligned { address } |  
        TrapCause::StoreAccessFault { address } => *address as u32,
        TrapCause::IllegalInstruction { instruction } => instruction.unwrap_or(0), 
        TrapCause::Breakpoint | TrapCause::EnvironmentCallFromMMode | TrapCause::EnvironmentCallFromSMode | TrapCause::EnvironmentCallFromUMode 
            => 0
    };
    cpu.csr.write(MTVAL, trap_val, CPUMode::M)?;
    Ok(())
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
    cpu.pc.write(cpu.csr.read(MTVEC) as usize);
}

fn set_mpp(cpu: &mut CPUState) -> Result<(), TrapCause> {
    let mstatus_addr = MSTATUS;
    let mstatus_state = cpu.csr.read(MSTATUS);
    let privilege_level = cpu.mode.as_privilege_level();
    let updated_mstatus = set_bit_range(mstatus_state, privilege_level, 2, 11);
    cpu.csr.write(MSTATUS, updated_mstatus, CPUMode::M);
    Ok(())
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
// a failure here is a double trap
pub fn handle_trap(cpu: &mut CPUState, trap_cause: TrapCause) {
    set_mepc(cpu); // store pc
    set_mpp(cpu); // save the mode to mstatus
    cpu.mode = CPUMode::M;
    set_mcause(cpu, trap_cause.mcause_code()); // store why the trap happened for guest function
    // "If mtval is written with a nonzero value when 
    // a breakpoint, address-misaligned, access-fault, page-fault, or hardware-error exception occurs 
    // on an instruction fetch, load, or store, 
    // then mtval will contain the faulting virtual address.
    set_mtval(cpu, &trap_cause);
    jump_to_trap_handler(cpu); // write the trap handler address to pc to go there
}

pub fn step(cpu: &mut CPUState) -> Result<ExecutionSignal, TrapCause> {
    match perform_step(cpu) {
        Ok(signal) => Ok(signal),
        Err(trap_cause) => {
            handle_trap(cpu, trap_cause);
            Ok(ExecutionSignal::Continue)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu_definition::build_cpu_state;
    use crate::utility::bit_operations::{store_in_mem, mask_and_shift};
    use crate::programs::instructions::ADD_X3_X1_X2;
    use crate::programs::instructions::NO_OP;
    use crate::definitions::masks;

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
        cpu.csr.write(MTVEC, 4, CPUMode::M);
        handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33) });
        let mpp = mask_and_shift(cpu.csr.read(MSTATUS), masks::MPP);
        assert_eq!(mpp, CPUMode::S.as_privilege_level());
        assert_eq!(cpu.mode, CPUMode::M);
    }

    #[test]
    fn test_handle_trap_changes_correct_values() {
        let mut cpu = build_cpu_state();
        cpu.pc.write(7);
        cpu.csr.write(MTVEC, 4, CPUMode::M);
        let outcome = handle_trap(&mut cpu, TrapCause::IllegalInstruction { instruction: Some(33)});
        assert_eq!(cpu.pc.read(), 4); // check MTVEC
        assert_eq!(cpu.csr.read(MEPC), 7); // mepc
        assert_eq!(cpu.csr.read(MCAUSE), 2); // mcause
        assert_eq!(cpu.csr.read(MTVAL), 33); // mtval
    }
}