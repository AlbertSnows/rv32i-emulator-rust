use rv32i_emulator::definitions::cpu::cpu_definition::{build_cpu_state};
use rv32i_emulator::elf::{load_elf, find_symbol};
use rv32i_emulator::core::step;

#[derive(Debug, PartialEq)]
pub enum TestOutcome {
    Pass,
    Fail(u32),
    TimedOut
}

pub const MAX_ITERATIONS: u32 = 10000;

pub fn run_tests(elf_bytes: &[u8]) -> TestOutcome {
    // let elf_bytes = todo!(); // get bytes from test file
    let mut cpu = build_cpu_state();
    load_elf(elf_bytes, &mut cpu).expect("load elf should succeed");
    let tohost_addr = find_symbol(elf_bytes, "tohost").expect("tohost should resolve") as usize;
    for _ in 0..MAX_ITERATIONS {
        let _ = step(&mut cpu);
        let tohost_value = cpu.bus.direct_read(tohost_addr, 4).unwrap();
        if tohost_value != 0 {
            if tohost_value == 1{
                return TestOutcome::Pass
            } else {
                // RVTEST_FAIL wrote (TESTNUM << 1) | 1 
                //  >> 1 reverses that: 
                // it discards whatever bit falls off the end, so it
                // doesn't matter that | 1 forced that bit to 1, 
                // and >> 1 / << 1 are exact inverses otherwise. Recovers which
                // TEST_RR_OP-numbered sub-test actually failed.
                let failing_subtest = tohost_value >> 1;
                return TestOutcome::Fail(failing_subtest)
            }
        }
    }
    TestOutcome::TimedOut
}

macro_rules! riscv_test {
    ($test_name:ident, $fixture_path:literal) => {
        #[test]
        fn $test_name() {
            let elf_bytes = include_bytes!($fixture_path);
            assert_eq!(run_tests(elf_bytes), TestOutcome::Pass);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // real riscv-tests boot code probes optional
    // CSRs (mnstatus, satp, pmpaddr0/pmpcfg0) by pointing mtvec at the next
    // instruction and deliberately triggering a trap if the CSR isn't
    // supported, then continuing without ever running MRET. 
    include!(concat!(env!("OUT_DIR"), "/generated_riscv_tests.rs"));
}
