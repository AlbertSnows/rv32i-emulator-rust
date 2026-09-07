# riscv-arch-test (ACT4) Setup

How to set up RISC-V International's own conformance suite,
`riscv-arch-test`, against this emulator. 

## Prerequisites

- `mise`,  if you don't already have it, see https://mise.jdx.dev.
- `make`, `git`,  standard, already present on most systems.
- A RISC-V GCC-compatible compiler on `PATH`,  the xPack
  `riscv-none-elf-gcc` from `riscv_toolchain_setup.md` works
- sail-riscv 0.13.1,  see `riscof_setup.md`; same install, still
  reused here.

## Steps

### 1. Clone into ~/opt

```bash
cd ~/opt
git clone, depth 1 https://github.com/riscv/riscv-arch-test.git
```

### 2. Trust the repo's mise config

```bash
cd ~/opt/riscv-arch-test
mise trust .mise.toml
```

### 3. Sanity-check: generate test assembly only

No compiler or sail needed for this step,  confirms `make`/`mise` are
working before anything else is involved.

```bash
make tests
```

On first run, `mise` auto-provisions everything the repo's `.mise.toml`
declares (Ruby, Bundler, `uv`, `prek`),  no manual installs needed.
Expect output ending in something like:

```
✓ Generated covergroups for 87 extension(s)
✓ Generated 188 test suite(s)
```

### 4. Build: scripts/build_arch_test.sh

The actual per-DUT config content is tracked in this repo, under
`tests/arch_test_config/`,  not only in `~/opt`, which isn't version
controlled. `~/opt/riscv-arch-test` only knows how to find config files
inside its own `config/` tree, so they still need to physically exist
there too. `scripts/build_arch_test.sh` handles the copy and the build
in one step (a POSIX shell script, so it runs the same regardless of
what shell invokes it,  no Nushell-vs-bash env-var syntax to worry
about):

```
./scripts/build_arch_test.sh
```

Defaults to `EXTENSIONS=I`; override with e.g.
`EXTENSIONS=I,M ./scripts/build_arch_test.sh`. Successful ELFs land in
`~/opt/riscv-arch-test/work/rv32i-emulator/elfs/`.

If you edit a file directly in `~/opt` while iterating on a build
error, copy the change back into `tests/arch_test_config/` afterward, 
the repo copy is the source of truth, the `~/opt` copy is just where
the build tool needs it to physically sit.

## Per-DUT config

### Why these files are needed

ACT4 doesn't generate one generic test suite,  it generates tests
*for a specific implementation*, then compiles a self-checking ELF
where the correct answers were computed by running sail-riscv
*configured to match that same implementation*. Every file in this
directory answers one part of "what does this specific implementation
look like":

- `rv32i-emulator.yaml` (UDB config),  which instructions/extensions
  exist at all, so ACT4 only generates tests this DUT could possibly
  pass.
- `sail.json`,  the same implementation facts, but in the format
  sail-riscv itself needs, so the reference run that computes expected
  answers uses the identical configuration.
- `link.ld`,  where memory actually lives on this DUT, so the compiled
  test ELF's addresses are ones this emulator can actually execute at.
- `rvmodel_macros.h`,  how this specific DUT boots, prints, and
  signals pass/fail/halt, since those aren't standardized across real
  hardware/emulators.
- `rvtest_config.h`/`rvtest_config.svh`,  C/SystemVerilog mirrors of
  the same extension facts as the UDB config, for template code that
  needs them at compile time rather than through UDB's own tooling.

Get any one of these wrong and the mismatch is exactly the kind
`AGENTS.md`'s Debugging section calls out first in its triage order:
"config/UDB mismatch, Sail config mismatch",  before ever suspecting
the DUT (this emulator) actually did something wrong.

A runnable config needs five files under
`~/opt/riscv-arch-test/config/cores/rv32i-emulator/rv32i-emulator/`:

- `rvmodel_macros.h`. `RVMODEL_DATA_SECTION`/`HALT_PASS`/`HALT_FAIL`
  content copied from `config/sail/sail-RVA23S64/rvmodel_macros.h`,
  since sail's own tohost write for termination is the same convention
  this project's riscv-tests harness already uses. Also defines
  `RVMODEL_IO_WRITE_STR` and all 8 interrupt SET/CLR macros as empty
  no-ops, plus `RVMODEL_INTERRUPT_LATENCY`/`TIMER_INT_SOON_DELAY` as
  plain numbers,  found via a real build failure that
  `check_defines.h` requires all of these to be *defined* regardless of
  whether this DUT actually supports IO/interrupts, contradicting the
  README's "can be left blank" wording.
- `test_config.yaml`. Fields per README's Configuration section;
  structure copied from `config/sail/sail-RVI20U32/test_config.yaml`
  with values swapped for this emulator's own tools (`riscv-none-elf-gcc`
  instead of `riscv64-unknown-elf-gcc`, confirmed working separately):
  ```yaml
  name: rv32i-emulator
  compiler_exe: riscv-none-elf-gcc
  objdump_exe: riscv-none-elf-objdump
  ref_model_exe: sail_riscv_sim
  udb_config: rv32i-emulator.yaml
  linker_script: link.ld
  dut_include_dir: .
  include_priv_tests: False
  ```
  `include_priv_tests: False` is temporary,  this emulator does
  implement M/S privilege, but starts unprivileged-only until the basic
  pipeline is proven end to end.
- `rv32i-emulator.yaml` (UDB config). First attempt trimmed
  `config/sail/sail-RVI20U32/sail-RVI20U32.yaml` (190 lines) down to
  just I/M/Zmmul/A/Zicsr/Zicntr, dropping F/D/C and the entire `Sm`
  extension plus its params on the assumption `Sm` only mattered for
  privileged *tests*. A real validation error proved that wrong:
  `MXLEN`/`PHYS_ADDR_WIDTH`/etc. are gated on `Sm` being *declared*,
  independent of `include_priv_tests`. Restored `Sm` and its full param
  block, plus F/D/C (over-declared, not implemented,  harmless for now
  since `EXTENSIONS=I` restricts what actually builds, but needs real
  trimming before an unrestricted build). Now validates and builds
  successfully.
- `link.ld`. Copied from `config/sail/sail-RVI20U32/link.ld` with two
  values changed to fit this emulator's real memory: `RAM_LENGTH` and
  `STACK_SIZE`. `RAM_ORIGIN` already matched `BASE_ADDRESS` exactly,
  left unchanged. Section layout/ordering is a framework structural
  requirement, not DUT-specific, copied unchanged.

  `RAM_LENGTH` went through two values before landing: first
  `0x10000` (matching `FULL_MEM_SIZE`'s original 65536 bytes), which a
  real build proved wrong,  even `I-auipc-00` alone needed ~94KB.
  Grown to `0x40000` (256KiB),  `FULL_MEM_SIZE` in
  `src/cpu/definitions/cpu/memory.rs` grown to match. `STACK_SIZE` cut
  to `0x1000` (the example's `0x20000` default alone was larger than
  even the original 64KiB memory).
- `sail.json`. Copied whole from `config/sail/sail-RVI20U32/sail.json`
  (845 lines; sail-riscv's own native config format, what
  `sail_riscv_sim, config` reads; uses `//` comments, non-standard
  JSON); only the RAM region's `size` field changed, kept in sync with
  `link.ld`'s `RAM_LENGTH` (now `0x40000`). Everything else copied
  as-is.
- `rvtest_config.h`/`rvtest_config.svh`,  NOT needed, despite both the
  README ("still a work in progress, need to be handwritten for now")
  and this repo's own AGENTS.md listing them as required. Checked the
  actual generation code (`framework/src/act/parse_udb_config.py:161-166`):
  both are auto-generated from the UDB yaml during the build. Confirmed
  by checking every example config directory in the repo, including
  the one the README itself cites (config/cores/cvw/cvw-rv64gc/),  none
  have these files on disk. Both docs are stale here.
- `run_cmd.txt`,  not one of the five README lists, but needed for a
  different reason: its mere presence makes the Makefile
  auto-generate a `make rv32i-emulator` build shortcut
  (`RUN_CMD_FILES := $(shell find config -name run_cmd.txt)`), and its
  content is the command `run_tests.py` uses to invoke the DUT against
  each generated ELF (path appended automatically). Currently just
  points at `target/debug/arch_test_runner`,  the *build* shortcut
  works already, but the *run* result won't be meaningful until
  `arch_test_runner.rs` is reworked to poll `tohost` (see
  `docs/plans/arch_test.md` step 6).
