// rvmodel_macros.h -- DUT-specific macros for rv32i-emulator.
//
// tests/env/check_defines.h errors at compile time if any macro below
// is missing -- confirmed by an actual build failure (I-auipc-00's
// self-check compilation) after leaving the IO/interrupt macros
// undefined per the README's "can be left blank" wording. That wording
// actually means "define it with an empty/no-op body," not "omit the
// #define entirely" -- check_defines.h has no #ifdef guard around most
// of these, they're unconditionally required regardless of what this
// DUT actually supports.
//
// RVMODEL_DATA_SECTION and RVMODEL_HALT_PASS/RVMODEL_HALT_FAIL content
// copied from config/sail/sail-RVA23S64/rvmodel_macros.h, minus
// CLINT_BASE_ADDRESS (sail-specific, this emulator doesn't need it) --
// both DUTs use the same tohost/HTIF termination convention this
// project's riscv-tests harness already implements.
//
// STANDARD_SM_SUPPORTED was *also* excluded on that same "sail-specific"
// assumption, which was wrong: it's not sail-specific at all -- it's a
// signal telling the shared framework code "this DUT correctly
// implements the standard machine-mode privileged architecture," and
// RVTEST_BOOT_TO_MMODE (tests/env/rvtest_setup.h) gates its entire
// trap-CSR setup block (including writing mtvec) behind
// `#ifdef STANDARD_SM_SUPPORTED`. Leaving it undefined meant mtvec was
// never initialized, causing sail_riscv_sim itself to hit an infinite
// fetch-access-fault loop at address 0 the first time boot code
// attempted a trap (an M-mode-to-self ecall, part of the framework's
// "T-SBI" mechanism) -- confirmed via --trace.
//
// Everything below TERMINATION is a required no-op: this emulator has
// no console and no interrupt injection yet, so IO_WRITE_STR and every
// SET/CLR_*_INT macro expand to nothing -- they just need to exist
// syntactically. RVMODEL_INTERRUPT_LATENCY/TIMER_INT_SOON_DELAY need
// real numeric values (copied from sail's own), even though nothing
// exercises timer interrupts yet, since RVMODEL_MTIME_ADDRESS isn't
// defined either.

#ifndef _RVMODEL_MACROS_H
#define _RVMODEL_MACROS_H

#define STANDARD_SM_SUPPORTED

#define RVMODEL_DATA_SECTION \
        .pushsection .tohost,"aw",@progbits;                \
        .balign 8; .global tohost; tohost: .dword 0;         \
        .balign 8; .global fromhost; fromhost: .dword 0;     \
        .popsection

#define RVMODEL_HALT_PASS  \
  li x1, 1                ;\
  la t0, tohost           ;\
  write_tohost_pass:      ;\
    sw x1, 0(t0)          ;\
    sw x0, 4(t0)          ;\
    j write_tohost_pass   ;\


#define RVMODEL_HALT_FAIL \
  li x1, 3                ;\
  la t0, tohost           ;\
  write_tohost_fail:      ;\
    sw x1, 0(t0)          ;\
    sw x0, 4(t0)          ;\
    j write_tohost_fail   ;\

#define RVMODEL_IO_WRITE_STR(_R1, _R2, _R3, _STR_PTR)               \
1:                                                                 ;\
  lbu _R1, 0(_STR_PTR)        /* load next byte */                ;\
  beqz _R1, 3f                /* exit loop if null terminator */  ;\
2:                                                                 ;\
  la _R2, tohost                                                  ;\
  li _R3, 0x01010000           /* device=1 (terminal), cmd=1 (output) -- write this marker BEFORE the character byte, so a host polling after every instruction never observes a nonzero low word paired with a stale (often zero) high word */ ;\
  sw _R3, 4(_R2)                                                  ;\
  sw _R1, 0(_R2)               /* write the character byte */     ;\
  addi _STR_PTR, _STR_PTR, 1                                      ;\
  j 1b                                                             ;\
3:

#define RVMODEL_INTERRUPT_LATENCY 1
#define RVMODEL_TIMER_INT_SOON_DELAY 100

#define RVMODEL_SET_MEXT_INT(_R1, _R2)
#define RVMODEL_CLR_MEXT_INT(_R1, _R2)
#define RVMODEL_SET_MSW_INT(_R1, _R2)
#define RVMODEL_CLR_MSW_INT(_R1, _R2)
#define RVMODEL_SET_SEXT_INT(_R1, _R2)
#define RVMODEL_CLR_SEXT_INT(_R1, _R2)
#define RVMODEL_SET_SSW_INT(_R1, _R2)
#define RVMODEL_CLR_SSW_INT(_R1, _R2)

#endif
