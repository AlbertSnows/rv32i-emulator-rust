use crate::definitions::cpu_definition::CPUState;
use crate::definitions::codes::ExecutionSignal;
use crate::fetcher::fetch_word_from_memory;
use crate::decoder::decode_word_to_instruction;
use crate::instructions::pc::advance_pc;

pub fn step(cpu: &mut CPUState) -> Result<ExecutionSignal, String> {
    // mut allows cpu to change in the local scope
    let raw_word = fetch_word_from_memory(&cpu.pc, &cpu.mem)?; // 51 = 0x33 = 0011 0011
    let instruction = decode_word_to_instruction(raw_word)?;
    // &mut cpu passes a mutable reference to cpu
    // &mut cpu = this reference has "mutable" permission to cpu
    let execution_outcome = instruction.execute(cpu)?;
    advance_pc(&mut cpu.pc, &instruction, &cpu.register);
    Ok(execution_outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::cpu_definition::build_cpu_state;
    use crate::utility::bit_operations::store_in_mem;
    use crate::programs::instructions::ADD_X3_X1_X2;
    use crate::programs::instructions::NO_OP;

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
    fn test_step_returns_err_on_undefined_opcode() {
        let mut cpu = build_cpu_state();
        // 0b0000000 isn't a real opcode -- decode should fail and step should
        // propagate that Err rather than panicking or silently continuing.
        store_in_mem(&NO_OP.to_le_bytes(), &mut cpu.mem, 0);
        let outcome = step(&mut cpu);
        assert!(outcome.is_err());
    }
}