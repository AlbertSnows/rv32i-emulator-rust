use std::path::Path;
use rv32i_emulator::core::step;
use rv32i_emulator::definitions::cpu::cpu_definition::build_cpu_state;
use rv32i_emulator::elf::{find_symbol, load_elf};

fn main() {
    //     3b — run until halt. A loop calling step(&mut cpu) repeatedly, same shape as run_tests in tests/harness.rs, but instead of polling tohost,
    // check whether pc stopped changing between iterations (the self-jump halt convention we settled on)
    // — record pc before each step(),
    // and if it's identical after, stop.
    // Bound it with a MAX_ITERATIONS-style cap too, same reasoning run_tests has one.
    //
    //     3c — dump the signature. find_symbol(&bytes, "begin_signature") and find_symbol(&bytes, "end_signature"), then walk that address range 4 bytes at a time with cpu.bus.direct_read(addr, 4), printing each word as format!("{:08x}", word) on its own line.
    //
    //     Since it's currently empty, 3a is the natural place to start — want to write that first piece?
    let elf_bytes = std::fs::read(std::fs::read(Path{inner: "path"})).unwrap();
    let mut cpu = build_cpu_state();
    load_elf(elf_bytes, &mut cpu).expect("load elf should succeed");
    let tohost_addr = find_symbol(elf_bytes, "tohost").expect("tohost should resolve") as usize;
    // for _ in 0..MAX_ITERATIONS {
    //     let _ = step(&mut cpu);
    //     let tohost_value = cpu.bus.direct_read(tohost_addr, 4).unwrap();
    //     if tohost_value != 0 {
    //         return if tohost_value == 1 {
    //             TestOutcome::Pass
    //         } else {
    //             // RVTEST_FAIL wrote (TESTNUM << 1) | 1
    //             //  >> 1 reverses that:
    //             // it discards whatever bit falls off the end, so it
    //             // doesn't matter that | 1 forced that bit to 1,
    //             // and >> 1 / << 1 are exact inverses otherwise. Recovers which
    //             // TEST_RR_OP-numbered sub-test actually failed.
    //             let failing_subtest = tohost_value >> 1;
    //             TestOutcome::Fail(failing_subtest)
    //         }
    //     }
    // }
    // TestOutcome::TimedOut
}
