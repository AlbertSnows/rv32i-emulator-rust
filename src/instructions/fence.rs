use crate::instructions::Format;
use crate::fetcher::InstructionWord;
use crate::definitions::trap_cause::TrapCause;
use crate::definitions::codes::ExecutionSignal;

// Table 72. RISC-V base opcode map, inst[1:0]=11
// inst[6:5] = 00, 01, 10, 11
// inst[4:2] = 000, ..., 111
// row/column
// MISC-MEM = row 00, column 011
// 6:0 = 00_011_11

pub fn parse_fence_inst(raw_word: InstructionWord) -> Result<Format, TrapCause> {
    Ok(Format::FENCEType)
}


pub fn execute_fence_type() -> Result<ExecutionSignal, TrapCause> {
    Ok(ExecutionSignal::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fence_inst() {
        let raw_word = InstructionWord(0b0001111);
        let result = parse_fence_inst(raw_word);
        assert_eq!(result, Ok(Format::FENCEType));
    }

    #[test]
    fn test_execute_fence_type() {
        let outcome = execute_fence_type();
        assert_eq!(outcome, Ok(ExecutionSignal::Continue));
    }
}
