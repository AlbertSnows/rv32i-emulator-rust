# riscv-arch-test adoption (item #13, second half)

## Why

riscv-tests is a per-instruction smoke test: does ADD compute the right
sum. riscv-arch-test is RISC-V International's own conformance suite —
broader edge-case coverage per instruction (boundary values, corner
cases a hand-picked smoke test wouldn't think to try), and it's the
actual standard a real implementation is checked against. Worth doing
precisely because riscv-tests already proved the basics work; this is
the next rung, not a replacement.

## PIVOT, 2026-08-30: riscof is deprecated, this now targets ACT4

Everything below this point through "history: the original RISCOF
plan" was built against RISCOF. Cloning the actual current
riscv-arch-test repo to check on `arch_test.h`'s real shape surfaced
that its own README says outright: "The Architectural Certification
Tests are used with the ACT4 Framework, a Makefile and Python based
tool that replaces the deprecated riscof tool." RISCOF and ACT4 are not
compatible drop-ins — different DUT contract, different execution model
entirely:

- **RISCOF**: run the same test live on the DUT and on sail-riscv, diff
  the two resulting signature files afterward.
- **ACT4**: sail-riscv runs once, offline, during test *generation* —
  its computed expected values get compiled directly into the test
  ELF, producing a self-checking binary. Just run that one ELF on the
  DUT; it reports its own pass/fail. No live second run, no post-hoc
  diff.

sail-riscv itself (0.13.1, prebuilt binary, already installed — see
`docs/dev/`) carries over unchanged; ACT4's README pins the exact same
version. Everything else RISCOF-specific (`config.ini`, the
`spike`/`sail_cSim` plugin scaffolding at the project root) is dead
weight now — fine to delete once ACT4 is actually working.

## 1. Toolchain still needed

- `make` — already present (`/usr/bin/make`).
- `mise` (recommended) or manually `uv` + Python 3.10+ + Ruby/Bundler —
  ACT4's test generator and framework are Python, and it also depends
  on riscv-unified-db (UDB), a Ruby gem, for DUT configuration schema.
  Neither installed yet.
- A RISC-V GCC compatible with ACT4 — its docs specifically reference
  `riscv64-unknown-elf-gcc` built via riscv-gnu-toolchain with a
  multilib config covering both rv32 and rv64 targets. The xPack
  `riscv-none-elf-gcc` already installed for riscv-tests may or may
  not be accepted (`test_config.yaml`'s `compiler_exe` is just an
  executable name/path, so pointing it at the xPack binary might just
  work) — check before assuming a second toolchain build is required.
- sail-riscv 0.13.1 — done already, no new work.

## 2. Get the real source

Clone `github.com/riscv/riscv-arch-test` to `~/opt/riscv-arch-test` —
NOT inside the project directory. First attempt put it at the project
root (gitignored, matching `.venv-riscof`); that was wrong by two
orders of magnitude — `.venv-riscof` is tens of MB, this clone is
16,449 files / 2.7GB (mostly generated `.S` tests under `tests/`), and
gitignore only stops it from being *committed*, not from being
*indexed* by an IDE watching the project directory. Dropping it into
the open project crashed RustRover (2026-08-30). External tool
dependencies this large belong outside the workspace entirely, same as
the compiler toolchain and sail-riscv already do (`~/opt/`). Needs
`mise trust .mise.toml` on first use.

## 3. Per-DUT config directory

Five-ish files, conventionally under `config/cores/<vendor>/<dut>/`
(e.g. `config/cores/rv32i-emulator/rv32i-emulator/`):

- `test_config.yaml` — tiny (~7 lines): name, `compiler_exe`,
  `objdump_exe`, `ref_model_exe` (`sail_riscv_sim`), and paths to the
  other three files.
- `<dut>.yaml` — UDB config declaring which extensions/params this
  emulator implements. `config/sail/sail-RVI20U32/` (RV32 base user
  profile, ~190 lines) is a much closer starting template than the
  larger RVA23S64 profile — still bigger than RISCOF's old `isa.yaml`,
  copy-and-trim rather than write from scratch.
- `rvmodel_macros.h` — only `RVMODEL_HALT_PASS`/`RVMODEL_HALT_FAIL` are
  actually required; everything else (IO/console, timer, interrupt
  macros) can be left blank given this emulator's current feature set
  (no console, no interrupt injection yet). Checked sail's own current
  example (`config/sail/sail-RVA23S64/rvmodel_macros.h`): both halt
  macros just write to `tohost` (1 = pass, 3 = fail) and spin — the
  exact convention this project already fully implements for
  riscv-tests. Zero new emulator-side mechanism needed for
  halt/pass-fail detection.
- `link.ld` — structural requirements (`ENTRY=rvtest_entry_point`,
  `.text.init` at `TEST_BASE`, specific section ordering,
  `__stack_bottom`/`__stack_top`, etc.) are spelled out in the README;
  the sail example (52 lines) already sets `RAM_ORIGIN = 0x80000000`,
  matching this project's own `BASE_ADDRESS` exactly — copy-and-adapt,
  not write-from-scratch.
- `sail.json`, `rvtest_config.svh`, `rvtest_config.h` — README calls
  these "will eventually be auto-generated ... but that is still a
  work in progress, so they need to be handwritten for now." `sail.json`
  is 845 lines but described as mirroring the UDB config's memory
  map — copy the sail example and adjust addresses/extensions to
  match.

## 4. Build, then run

`CONFIG_FILES=<dir>/test_config.yaml make --jobs N` generates test
assembly, compiles a signature-generating variant, runs *that* on
`sail_riscv_sim` (this is the one place sail actually executes — once,
per test, at build time, not something this emulator's runner ever
invokes), then recompiles a final self-checking ELF with the expected
values baked in, landing in `$WORKDIR/<config>/elfs/`.

Running those ELFs on this emulator needs no new mechanism: since
`RVMODEL_HALT_PASS`/`FAIL` reuse the `tohost` convention,
`arch_test_runner.rs` should end up looking like `tests/harness.rs`'s
`run_tests` (load ELF, bounded `step()` loop, poll `tohost`, decide
Pass/Fail/TimedOut) — not the signature-dump shape it was built as
under the RISCOF plan. The `begin_signature`/`end_signature` dump logic
already written isn't what ACT4 needs; the self-checking ELF already
knows the right answer.

