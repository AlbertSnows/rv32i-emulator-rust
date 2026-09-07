# Dev Notes

## Prerequisites

- Rust toolchain (rustup/cargo) — already installed, see `cargo --version`


## Dev loop

```
bacon run
```

^ -> Allows you to essentially hot reload changed files

## Build & Run

```
cargo build
cargo run
```

## Test

```
cargo test
```

Runs unit tests and riscv-tests (both wired into the same `cargo test`
run). Doesn't touch riscv-arch-test — that's a separate, external
framework (`~/opt/riscv-arch-test`, see
`docs/dev/riscv_arch_test_setup.md`) not invoked by `cargo test` at
all.

### Full verification (all three test layers)

```
scripts/verify.sh
```

Runs, in order, exiting on the first failure: `cargo test` (unit tests
+ riscv-tests), then builds `arch_test_runner`, then runs
riscv-arch-test via `scripts/build_arch_test.sh`. Exit code reflects
whichever step failed — nothing is summarized or interpreted, the
script just runs the real commands and lets each one's own exit code
propagate.

`EXTENSIONS=<suites> scripts/verify.sh` restricts which arch-test
suites run (default: the full currently-declared set, `I,M,Zmmul,A,
Zicntr,Zicsr,Zifencei,Zaamo,Zalrsc`) — same override
`scripts/build_arch_test.sh` itself takes.

This exists so a "N/N passing" claim (in a doc, in conversation,
anywhere) can be independently re-run rather than taken on faith —
run it yourself rather than trusting a reported number.

## Project Structure

The organizing idea is a strict pipeline, and the source layout mirrors
it: **fetch -> decode -> execute**, plus everything around that
pipeline kept out of it.

- **Fetch** (`fetcher.rs`) just reads a 16- or 32-bit word from memory
  at the current PC — it doesn't know what the bits mean.
- **Decode** (`decoder.rs`, `instructions/*.rs`) turns that raw word
  into one variant of a single `Format` enum. Decoding is organized by
  *wire shape*, not by ISA extension — there's an `r.rs`, an `s.rs`, a
  `c.rs`, but no "M extension" file, because `M`'s instructions are
  just more `RType`s and don't need their own category. This is also
  why the `C` extension needed no new execution code at all: its whole
  job is producing the same `Format` variants the 32-bit decoder
  already does, just from 16-bit input.
- **Execute** (the `execute_*` function next to each `parse_*`) takes a
  decoded `Format` and mutates `CPUState`. `core.rs` is the only place
  that ties all three stages together into one step, plus everything
  that isn't strictly fetch/decode/execute: trap handling and interrupt
  selection.

One wrinkle in that scheme: I-type is a single 32-bit wire encoding but
gets split into six different `Format` variants (loads, ALU-immediate,
`JALR`, shifts, system/CSR instructions). Decode shape and execution
*meaning* are different axes — several instructions can agree on the
former while having nothing in common in the latter, and the split
follows meaning, not wire shape.

Everything decode and execute depend on but that isn't logic itself —
CSR addresses, bit masks, opcodes, the `TrapCause` enum, `CPUState`'s
actual field layout — lives under `definitions/`, so a mask or address
constant is defined once rather than copied into every instruction
file that needs it.

`CPUState` is the one mutable object threaded through every step
(`register`, `pc`, `csr`, `bus`, `mode`, `flags`). Nothing outside
`bus.rs` talks to memory-mapped peripherals directly — RAM, the UART,
the PLIC all sit behind one address-range dispatcher, so the CPU core
itself never special-cases a specific device. The peripherals
themselves (`peripherals/`) are a separate concern entirely from the
CPU: they model real external hardware (deliberately at the same
addresses real QEMU uses), not anything about instruction execution.

`loader.rs` is its own boundary, separate from both: it's about how
content gets *into* memory in the first place (an ELF, or the
OpenSBI+kernel+DTB trio for a real Linux boot) — a concern the CPU
core has no reason to know about. `utility/` sits underneath
everything as low-level helpers (bit masking, sign-extension, byte
sizes) with no RISC-V-specific knowledge at all.

Above the library, `src/bin/` holds three small binaries rather than
one configurable tool, each a different front end onto the same
`rv32i_emulator` library crate: an interactive one (`run_os`), an
instrumented/diagnostic one (`debug_boot`), and a headless conformance
-test driver (`arch_test_runner`). `tests/`, `scripts/`, and `docs/`
mirror that same "keep concerns separate" instinct one level up —
three independent test layers (unit tests, riscv-tests,
riscv-arch-test) rather than one, scripts that wrap a manual recipe
into one command instead of the recipe living only in someone's
memory, and docs split by *intent* (`definitions/` for spec/concept
reference, `dev/` for operational runbooks, `plans/` for design
rationale and status, `research/` for raw unedited notes) rather than
one growing pile.

## Design Decisions

- **Every instruction, 16-bit or 32-bit, decodes onto one shared `Format`
  enum.** The `C` extension's entire implementation is decode-time
  expansion onto the same variants the base 32-bit decoder already
  produces — zero new execution code (`docs/plans/c_extension.md`).
  Adding an extension means teaching the decoder a new way to *produce*
  a `Format`, not teaching the CPU a new way to *run* one.

- **I-type is one wire encoding but six `Format` variants**
  (`LoadType`/`AluImmType`/`JalrType`/`IShiftType`/`SystemType`/
  `CsrType`). Decode shape and execution semantics are different axes;
  splitting on the latter keeps each `execute_*` function about one
  thing instead of one giant match on what the instruction "really" is.

- **All memory-mapped I/O goes through one `BUSState` dispatcher**
  (`bus.rs`), keyed by address range. The CPU core (`core.rs`,
  `fetcher.rs`) never special-cases a specific peripheral — it just
  reads/writes through the bus, the same as real hardware would.

- **Peripheral addresses deliberately match real QEMU's `virt` machine**
  (PLIC, UART, CLINT-equivalent — see `docs/dev/peripherals_specs.md`,
  `peripherals_no_spec.md`), so the exact same OpenSBI/Linux/DTB built
  for real QEMU boots unmodified here, and real QEMU + GDB can be used
  to cross-check this emulator's behavior when something's wrong
  (`docs/dev/boot_files_setup.md`'s "Cross-checking against real QEMU"
  section).

- **No FPU; every other base extension needed for Linux is implemented**
  (`M`, `A`, `C`, `Zicsr`, `Zifencei`, S/U privilege modes, Sv32). `F`/`D`
  exclusion is a stated scope boundary, not a gap being hidden — OpenSBI,
  the kernel, and the arch-test DUT config all declare this explicitly
  rather than silently omitting it.

- **Three independent, deliberately different-shaped correctness
  layers**, not one: hand-written unit tests (fast, instruction-level,
  written alongside each decoder), riscv-tests (the official
  per-instruction smoke suite), riscv-arch-test/ACT4 (the official
  edge-case conformance suite). They catch different bug classes on
  purpose 
  `scripts/verify.sh` runs all three so a "passing" claim can be
  re-checked.

- **External toolchains and build artifacts live outside the repo**
  (`~/opt/`) and are never committed — the RISC-V cross toolchains,
  OpenSBI, the Linux kernel, sail-riscv, riscv-arch-test. Each is fully
  reproducible from a documented recipe under `docs/dev/`, not vendored.

- **Acknowledged wart: `CPUFlags::in_trap` is written but nothing reads
  it anymore.** It used to gate interrupt delivery in
  `select_pending_interrupt` — a real bug, since a single boolean can't
  represent legitimate nested traps (an S-mode handler's own `ecall`
  into M-mode is itself a nested trap), and that gate let a second
  interrupt corrupt shared `sepc`/`scause` state the first time a real
  PLIC-routed interrupt occurred. `sstatus.SIE`/`mstatus.MIE` (already
  cleared correctly on trap entry) turned out to be the actual,
  sufficient, already-implemented protection real hardware relies on —
  removing the gate fixed it. The field itself is still set by
  `handle_trap`/`inst_i_xret` and asserted on by a few tests, but
  nothing reads it — an open decision in
  `docs/plans/refactor_and_revisit.md` §1 on whether to delete it
  entirely.

- **Acknowledged wart: `CSRState`'s read/write path has accumulated
  real special-casing** (per-address hardwired-bit masking, an internal
  bypass for cycle/instret that skips `write()` entirely, `MTVEC`'s
  mode-forcing). Tracked as a real refactor item
  (`docs/plans/plans.md` #14, `docs/plans/refactor_and_revisit.md` §3),
  not something the current shape pretends isn't there — the discarded
  -write bugs found in the `MIP`/`SIP` CSR arms this session are direct,
  concrete evidence of the cost of that special-casing.

- **`src/bin/` holds three thin binaries over one shared library**, not
  one do-everything tool: `run_os` (real interactive boot), `debug_boot`
  (the same boot with flag-gated diagnostics, built for the interrupt
  -nesting investigation and generalized since), `arch_test_runner`
  (headless, `tohost`-polling, drives both official conformance
  suites). Each links `rv32i_emulator` as an external crate rather than
  being folded into the library itself.

- **`src/cpu/programs/` predates the real decoder and ELF loader** —
  early hand-encoded instruction-word constants (`ADD_X3_X1_X2`,
  `JALR_X1_X1_0`), kept because some existing tests still construct
  input that way. Not the current recommended way to write a new test;
  new tests should build a real ELF fixture or decode from an actual
  encoding.


