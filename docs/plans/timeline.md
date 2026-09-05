# Timeline

The order major pieces of this project were built in
dates below, just what came before what.

1. **Base RV32I**: fetch/decode/execute for the six base instruction
   formats (R/I/S/B/U/J), starting entirely in M-mode.
2. **Hardware modes** (U/S/M) added to CPU state, still defaulting to
   M-mode at reset.
3. **Interrupts**: `mie`/`mip`, trap handling.
4. **Counters**: `cycle`/`instret`/`time` (`Zicntr`).
5. **Adopted riscv-tests** (the official per-instruction conformance
   suite) plus the toolchain and ELF loader needed to build and run
   it, replacing ad hoc hand-written instruction tests as the main
   correctness signal. Started running independently-authored test
   binaries against the base ISA for the first time surfaced several
   real bugs existing hand-written tests had never caught, because
   those tests shared the same wrong assumptions as the code they were
   testing (see `docs/plans/bug_log.md`).
6. **M extension** (multiply/divide) — needed because ordinary
   compiled C code assumes it exists.
7. **A extension** (atomics) — same reasoning as M; Linux's riscv port
   hardcodes atomics even on a single-hart build.
8. **Privileged mode / CSR conformance**: full S-mode CSR set, `SRET`,
   trap delegation (`medeleg`/`mideleg`) — the mechanism an S-mode
   kernel under M-mode firmware actually runs on.
9. **Sv32 virtual memory**: `satp`, the two-level page-table walker —
   without this there's no path to booting a real kernel at all, only
   bare-metal test programs.
10. **Adopted riscv-arch-test (ACT4)**: RISC-V International's own
    conformance suite, broader edge-case coverage than riscv-tests'
    per-instruction smoke tests.
11. **Peripherals / BIOS / OS phase begins**, deliberately modeled on
    QEMU's `virt` machine rather than designed from scratch: UART,
    PLIC, a multi-image loader for OpenSBI + kernel + DTB, matching
    QEMU's addresses and boot handoff so real, unmodified firmware and
    kernels work with no bespoke porting.
12. **Booted OpenSBI, then an unmodified Linux kernel**, to the same
    end state real hardware reaches with no root filesystem
    (`VFS: Unable to mount root fs`).
13. **C extension** (compressed instructions): reaching an actual
    shell needs a real root filesystem (an initramfs with a
    statically-linked busybox), and every real Linux userspace
    toolchain assumes compressed instructions exist — unlike
    OpenSBI/the kernel, which could be told not to use them since this
    project controls their builds.
14. **Reached an interactive shell**. Wiring up UART input, a stdin
    -reader thread, and the LSR data-ready bit surfaced one more real,
    deep bug: a single boolean (`in_trap`) meant to guard against
    re-entrant interrupts can't represent legitimate nested traps, and
    it let a second interrupt corrupt shared trap-CSR state the moment
    a real PLIC-routed interrupt occurred for the first time. Removing
    that gate (real hardware's own `sstatus.SIE`/`mstatus.MIE` checks
    already do the job correctly) fixed it — `busybox`'s `ash` reaches
    a `~ #` prompt and runs commands, piped or typed live. This was the
    project's original goal.
15. **Post-shell cleanup and hardening**: simplified the OpenSBI
    /kernel/musl build now that `C` no longer needs avoiding, fixed two
    more real CSR/fault-labeling bugs found along the way, added the
    riscv-tests compressed-instruction fixture and a script to automate
    building new ones, extended ACT4 to cover `C`/`Zca` — which caught
    a genuine spec bug unit tests and riscv-tests had both missed
    (`C.LUI` with `rd=x0` is a HINT, not an illegal encoding).

