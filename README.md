# rv32i-emulator

A RISC-V RV32I emulator written from scratch in Rust. It boots an unmodified Linux kernel through OpenSBI to an interactive busybox shell, entirely inside a software-interpreted CPU.

## Highlights

- **Full base RV32I** fetch/decode/execute, plus the `M` (multiply/divide), `A` (atomics), and `C` (compressed instructions) extensions
- **Privileged architecture**: M/S/U modes, the full S-mode CSR set, trap delegation (`medeleg`/`mideleg`), and `SRET`
- **Sv32 virtual memory**: `satp`, two-level page-table walker, page faults, the machinery a real kernel needs for per-process address spaces and demand paging
- **Peripherals modeled directly on QEMU's `virt` machine** (UART, PLIC), at the same addresses, so real, unmodified firmware and kernels boot with no bespoke porting
- **Boots into kernel**: an unmodified OpenSBI firmware image, then an unmodified Linux v6.1 kernel, to a working `ash` prompt where you can type or pipe commands live
- **Three independent conformance layers**: hand-written unit tests, the official `riscv-tests` per-instruction suite (81 fixtures), and RISC-V International's own `riscv-arch-test` (ACT4) edge-case suite, run together by one script

## Quick start

```bash
cargo build
cargo test          # unit tests + riscv-tests, both wired into `cargo test`
cargo run           # runs a small hardcoded demo program (src/cpu/main.rs)
```

Three binaries exist over the same `rv32i_emulator` library crate:

| Binary | What it does |
|---|---|
| `rv32i-emulator` | Small hardcoded demo program |
| `run_os` | Boots the real OS: OpenSBI + Linux + DTB, wires your terminal to the emulated UART |
| `debug_boot` | Same boot as `run_os`, with flag-gated tracing (`--symbols`, `--trace-traps`, `--trace-plic`) for diagnosing a stuck or faulting boot |
| `arch_test_runner` | Headless conformance driver: loads one ELF, polls `tohost`, reports pass/fail |

## Running Linux

`run_os` needs three external build artifacts it doesn't ship: an OpenSBI firmware image, a Linux `Image`, and a device tree blob. They're left out of the repo deliberately, they're large, fully reproducible build outputs, not source, the same reasoning that keeps the RISC-V toolchain and `sail-riscv` out too.

Full, step-by-step build instructions live in **[`docs/dev/boot_files_setup.md`](docs/dev/boot_files_setup.md)**. 
Once the three files exist, point `loader.rs`'s `boot_kernel` paths at them and run 
`cargo run --release --bin run_os`.

## Testing and conformance

```bash
scripts/verify.sh
```

This script runs all three test layers in order, unit tests and `riscv-tests`, 
then `riscv-arch-test`, stopping at the first failure and letting each tool's own exit code 
propagate rather than summarizing anything. Default extension coverage: 
`I,M,Zmmul,A,Zicntr,Zicsr,Zifencei,Zaamo,Zalrsc` (override with `EXTENSIONS=...`).

## Project structure

The source layout mirrors the CPU's own pipeline:

```
src/
  cpu/
    fetcher.rs        reads a 16- or 32-bit word at PC, doesn't interpret it
    decoder.rs, 
    instructions/  raw word -> one variant of a shared Format enum
    core.rs            ties fetch/decode/execute together, plus trap/interrupt handling
    mmu.rs             Sv32 page-table walker
    trap.rs            trap entry/exit, CSR delegation
    definitions/        CSR addresses, bit masks, opcodes, CPUState layout
  peripherals/         UART, PLIC, device tree, SBI, all behind one bus dispatcher
  loader.rs            gets content into memory: ELF, or OpenSBI+kernel+DTB
  utility/             RISC-V-agnostic helpers (bit ops, sign extension)
src/bin/               three thin front ends onto the library above
tests/                 riscv-tests fixtures + the harness that runs them
docs/
  dev/                 operational runbooks (toolchain setup, boot files, this structure doc)
  plans/               design rationale, build timeline, per-extension notes, bug log
  definitions/          spec/concept reference
  research/             raw notes
```

The full design-decisions writeup, including which tradeoffs were deliberate and which warts are known and tracked, lives in **[`docs/dev/README.md`](docs/dev/README.md)**.

## How this was built

Roughly in order: base RV32I in M-mode only, then U/S/M modes, interrupts, counters, 
doption of `riscv-tests` for correctness, `M` and `A` extensions, full privileged-mode CSR 
conformance, Sv32 virtual memory, `riscv-arch-test` adoption, then a peripherals/BIOS/OS 
phase modeled on QEMU's `virt` machine that ends with booting OpenSBI and an unmodified Linux 
kernel, the `C` extension (needed because every real userspace toolchain assumes compressed 
instructions exist), and finally a working interactive shell.

