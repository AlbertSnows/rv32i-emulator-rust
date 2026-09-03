# Adding riscv-tests Fixtures

How the existing `tests/fixtures/*-p-*` ELF binaries were built, and how
to add more (e.g. picking up the next ISA extension's conformance
tests).

## Where the sources come from

The [`riscv-tests`](https://github.com/riscv-software-src/riscv-tests)
repo has one subfolder per extension (`rv32ui`, `rv32um`,
`rv32ua`, ...), each with one `.S` file per instruction plus a
`Makefrag` listing which instructions belong to that extension.

## The build recipe

`riscv-tests/isa/Makefile` defines the actual compile rule (search for
`compile_template` and the `$(eval $(call compile_template,rv32um,...))`
line at the bottom to find the flags for a given extension). Pulled out
into a standalone command, building one instruction's `.S` file looks
like this — run from inside the `riscv-tests` checkout:

```bash
GCC=~/opt/xpack-riscv-none-elf-gcc-<VERSION>/bin/riscv-none-elf-gcc

$GCC -march=rv32g -mabi=ilp32 -static -mcmodel=medany -fvisibility=hidden \
  -nostdlib -nostartfiles \
  -Ienv/p -Iisa/macros/scalar -Tenv/p/link.ld \
  isa/<extension>/<instruction>.S \
  -o <instruction>-p-<instruction>
```

- `-march=rv32g -mabi=ilp32` — the flags `isa/Makefile` uses for every
  `rv32u*` extension (`g` already implies `m`/`a`/`f`/`d`, so this
  doesn't need to change per-extension).
- `-Ienv/p -Tenv/p/link.ld` — the "physical" (`p`) test environment:
  no OS, a fixed linker script, and `tohost`/`fromhost` signaling (see
  `docs/plans/test_suite.md` for what that protocol is and why the
  loader/harness care about the `tohost` symbol specifically).
- `-Iisa/macros/scalar` — pulls in `riscv_test.h` and the
  `TEST_*` assembly macros the `.S` files are written against.
- See `docs/dev/riscv_toolchain_setup.md` for how the toolchain
  (`riscv-none-elf-gcc`) itself gets installed.

Output filename convention: **`<name>-p-<name>`**, matching the
existing fixtures (`add-p-add`, `mul-p-mul`, ...) — `build.rs` derives
the generated test's name from the part before `-p-`, so this isn't
just cosmetic.

## Adding a whole extension at once

To pull in every instruction from one extension's `Makefrag`:

```bash
cd ~/Documents/programming/riscv/riscv-tests
GCC=~/opt/xpack-riscv-none-elf-gcc-<VERSION>/bin/riscv-none-elf-gcc
DEST=~/Documents/programming/rv32i-emulator/tests/fixtures

for t in <instruction names from the extension's Makefrag>; do
  $GCC -march=rv32g -mabi=ilp32 -static -mcmodel=medany -fvisibility=hidden \
    -nostdlib -nostartfiles \
    -Ienv/p -Iisa/macros/scalar -Tenv/p/link.ld \
    isa/<extension>/$t.S -o "$DEST/$t-p-$t"
done
```

(e.g. for `rv32um`: `mul mulh mulhsu mulhu div divu rem remu`, straight
out of `isa/rv32um/Makefrag`.)

## Wiring them into `cargo test`

Drop the built binaries into `tests/fixtures/`,
and `build.rs` picks up every file in that directory automatically
(`fs::read_dir(fixtures_dir)`), generating one `riscv_test!` per file
via `build.rs`'s `println!("cargo:rerun-if-changed=tests/fixtures")`.
Just run `cargo test`.

**Known wart:** `build.rs` currently hardcodes the generated test name
as `test_rv32ui_p_{name}_passes` regardless of which extension the
fixture actually belongs to — so e.g. the `rv32um` fixtures show up as
`test_rv32ui_p_mul_passes`, not `test_rv32um_p_mul_passes`. Harmless
(tests still run and mean the same thing), just mislabeled. 