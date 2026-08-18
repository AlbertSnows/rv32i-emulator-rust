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
    riscv_test!(test_rv32ui_p_add_passes, "fixtures/add-p-add");
    riscv_test!(test_rv32ui_p_addi_passes, "fixtures/addi-p-addi");
    riscv_test!(test_rv32ui_p_beq_passes, "fixtures/beq-p-beq");
    riscv_test!(test_rv32ui_p_and_passes, "fixtures/and-p-and");
    riscv_test!(test_rv32ui_p_andi_passes, "fixtures/andi-p-andi");
    riscv_test!(test_rv32ui_p_auipc_passes, "fixtures/auipc-p-auipc");
    riscv_test!(test_rv32ui_p_bge_passes, "fixtures/bge-p-bge");
    riscv_test!(test_rv32ui_p_bgeu_passes, "fixtures/bgeu-p-bgeu");
    riscv_test!(test_rv32ui_p_blt_passes, "fixtures/blt-p-blt");
    riscv_test!(test_rv32ui_p_bltu_passes, "fixtures/bltu-p-bltu");
    riscv_test!(test_rv32ui_p_bne_passes, "fixtures/bne-p-bne");
    riscv_test!(test_rv32ui_p_fence_i_passes, "fixtures/fence_i-p-fence_i");
    riscv_test!(test_rv32ui_p_jal_passes, "fixtures/jal-p-jal");
    riscv_test!(test_rv32ui_p_jalr_passes, "fixtures/jalr-p-jalr");
    riscv_test!(test_rv32ui_p_lb_passes, "fixtures/lb-p-lb");
    riscv_test!(test_rv32ui_p_lbu_passes, "fixtures/lbu-p-lbu");
    riscv_test!(test_rv32ui_p_ld_st_passes, "fixtures/ld_st-p-ld_st");
    riscv_test!(test_rv32ui_p_lh_passes, "fixtures/lh-p-lh");
    riscv_test!(test_rv32ui_p_lhu_passes, "fixtures/lhu-p-lhu");
    riscv_test!(test_rv32ui_p_lui_passes, "fixtures/lui-p-lui");
    riscv_test!(test_rv32ui_p_lw_passes, "fixtures/lw-p-lw");
    riscv_test!(test_rv32ui_p_ma_data_passes, "fixtures/ma_data-p-ma_data");
    riscv_test!(test_rv32ui_p_or_passes, "fixtures/or-p-or");
    riscv_test!(test_rv32ui_p_ori_passes, "fixtures/ori-p-ori");
    riscv_test!(test_rv32ui_p_sb_passes, "fixtures/sb-p-sb");
    riscv_test!(test_rv32ui_p_sh_passes, "fixtures/sh-p-sh");
    riscv_test!(test_rv32ui_p_simple_passes, "fixtures/simple-p-simple");
    riscv_test!(test_rv32ui_p_sll_passes, "fixtures/sll-p-sll");
    riscv_test!(test_rv32ui_p_slli_passes, "fixtures/slli-p-slli");
    riscv_test!(test_rv32ui_p_slt_passes, "fixtures/slt-p-slt");
    riscv_test!(test_rv32ui_p_slti_passes, "fixtures/slti-p-slti");
    riscv_test!(test_rv32ui_p_sltiu_passes, "fixtures/sltiu-p-sltiu");
    riscv_test!(test_rv32ui_p_sltu_passes, "fixtures/sltu-p-sltu");
    riscv_test!(test_rv32ui_p_sra_passes, "fixtures/sra-p-sra");
    riscv_test!(test_rv32ui_p_srai_passes, "fixtures/srai-p-srai");
    riscv_test!(test_rv32ui_p_srl_passes, "fixtures/srl-p-srl");
    riscv_test!(test_rv32ui_p_srli_passes, "fixtures/srli-p-srli");
    riscv_test!(test_rv32ui_p_st_ld_passes, "fixtures/st_ld-p-st_ld");
    riscv_test!(test_rv32ui_p_sub_passes, "fixtures/sub-p-sub");
    riscv_test!(test_rv32ui_p_sw_passes, "fixtures/sw-p-sw");
    riscv_test!(test_rv32ui_p_xor_passes, "fixtures/xor-p-xor");
    riscv_test!(test_rv32ui_p_xori_passes, "fixtures/xori-p-xori");
}
