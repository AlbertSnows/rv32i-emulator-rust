# Boot files setup: OpenSBI, Linux kernel, device tree

How to build the three files `loader.rs`'s `boot_kernel` needs: an
OpenSBI firmware image, a Linux kernel `Image`, and a device tree blob
(DTB). See `docs/dev/multi_image_loader.md` for what each one is for
and how the handoff between them works.

## Why these aren't committed to the repo

All three are large, fully reproducible build artifacts, not hand
-written source — same reasoning as the RISC-V toolchain and
sail-riscv already living outside the repo. Build output paths for
each should be added to `.gitignore` rather than committed.

## Prerequisites

- The bare-metal RISC-V toolchain from `docs/dev/riscv_toolchain_setup.md`
  (`riscv-none-elf-gcc`), already on `PATH`.
- `qemu-system-riscv32`, for generating the DTB — on Fedora Silverblue
  (no `dnf` on the host), install via `toolbox` or
  `rpm-ostree install qemu-system-riscv`.
- `~/opt/` as the install location, matching this project's existing
  convention for external tooling.
- **All commands below assume a `toolbox` session, whose default shell
  is bash** — regardless of what your host's interactive shell is (this
  project's host shell is Nushell; don't mix the two up, `export
  VAR=value` is a parser error in Nushell, and Nushell's `$env.VAR =
  value` is a parser error in bash).

## 1. Build OpenSBI

Source: [`riscv-software-src/opensbi`](https://github.com/riscv-software-src/opensbi).
Using the `generic` platform — the same one QEMU's `virt` machine uses
(confirmed in OpenSBI's own [`docs/platform/generic.md`](https://github.com/riscv-software-src/opensbi/blob/master/docs/platform/generic.md),
which lists "QEMU RISC-V Virt Machine" as a `generic`-platform user) —
means no custom platform code is needed.

**The bare-metal toolchain (`riscv-none-elf-gcc`) does not work here** —
confirmed directly from OpenSBI's own [`README.md`](https://github.com/riscv-software-src/opensbi/blob/master/README.md):
"Toolchains with Position Independent Executable (PIE) support like
*riscv64-linux-gnu-gcc*... or *Clang/LLVM* are required... **Bare-metal
GNU toolchains (e.g. *riscv64-unknown-elf-gcc*) cannot be used.**"
OpenSBI needs to produce a PIE (position-independent) firmware image so
it can run at a runtime-determined address, and bare-metal toolchains
generally don't support that at all — attempting it fails at configure
time with `Your linker does not support creating PIEs`.

The fix is `clang`/LLVM instead, which the same README confirms can
still produce PIE images for a bare-metal target. Since this project's
host is Fedora Silverblue (no `dnf` on the host), install `clang`+`lld`
in a `toolbox` rather than layering them onto the host. **A `toolbox`
session's default shell is bash, regardless of what your host's
interactive shell is** (this project's host shell is Nushell — don't
confuse the two; every command below runs inside the toolbox, in bash):

```bash
toolbox create riscv-build   # if you don't already have one
toolbox enter riscv-build
sudo dnf install clang lld llvm   # llvm specifically for llvm-ar
```

Then, inside the toolbox:

```bash
git clone https://github.com/riscv-software-src/opensbi ~/opt/opensbi
cd ~/opt/opensbi
export CROSS_COMPILE=riscv32-unknown-elf-
export PLATFORM_RISCV_XLEN=32
make PLATFORM=generic FW_DYNAMIC=y LLVM=1
```

A few things about this that aren't obvious from the flags alone, each
a real error hit while writing this doc:

- **`CROSS_COMPILE` doesn't need to point at a real binary here.** In
  `LLVM=1` mode, OpenSBI's `Makefile` strips the trailing `-` and passes
  the rest (`riscv32-unknown-elf`) to `clang` as a `--target` triple,
  rather than using it as an executable name prefix — nothing named
  `riscv32-unknown-elf-clang` needs to exist. Leaving `CROSS_COMPILE`
  unset entirely makes `clang` default to compiling for the *host*
  machine (x86_64) instead of RISC-V (visible as a `-mabi=lp64`/
  `-mcmodel=medany` error — those are RISC-V-only flags; seeing "valid
  arguments... are: ms sysv" back from the compiler is the tell that it
  silently compiled for x86_64 instead).
- **The triple must say `riscv32`, not `riscv64`, and must match
  `PLATFORM_RISCV_XLEN`.** Using `riscv64-unknown-elf-` here while
  `PLATFORM_RISCV_XLEN=32` produces 32-bit object files that the linker
  then tries to link as 64-bit RISC-V ELF (`ld.lld` fails with
  `is incompatible with elf64lriscv`) — the target triple controls the
  *linker's* bit-width expectation independently of what
  `PLATFORM_RISCV_XLEN` tells the *compiler* to generate; they have to
  agree.
- `llvm-ar` (from the separate `llvm` package, not just `clang`+`lld`)
  is also required — `AR` defaults to it in `LLVM=1` mode, and its
  absence fails with a plain `command not found` partway through the
  build, after several files have already compiled successfully.
- **`LLVM=1` must be passed on *every* invocation, including `make
  clean`** — it's a per-invocation variable, not a persistent setting.
  Running plain `make clean` (without `LLVM=1`) makes the Makefile fall
  back to its non-LLVM default (`$(CROSS_COMPILE)gcc`), which then goes
  looking for a real binary literally named `riscv32-unknown-elf-gcc`
  and fails with `command not found`.
- **If you change `CROSS_COMPILE` between build attempts, run `make
  LLVM=1 clean` first.** `make` only tracks source-file timestamps, not
  which flags a previous build used — switching from a 64-bit to a
  32-bit target without cleaning first leaves stale 64-bit `.o` files
  sitting in `build/`, which then fail to link against newly-compiled
  32-bit ones (`ld.lld: ... is incompatible with elf32lriscv` or
  `elf64lriscv`, depending on which direction the mismatch goes).

Output: `build/platform/generic/firmware/fw_dynamic.elf` — this is the
exact path currently hardcoded as `sbi_path` in `loader.rs`.

## 2. Build the Linux kernel

Source: [`torvalds/linux`](https://github.com/torvalds/linux). Mainline has no
single "rv32 defconfig" — [`arch/riscv/configs/`](https://github.com/torvalds/linux/tree/master/arch/riscv/configs)
ships a base `defconfig` plus a `32-bit.config` fragment, merged with
the kernel's own config-merging script.

Also inside the toolbox (bash) — **note the different `CROSS_COMPILE`
value from the OpenSBI step**: the kernel build has no `LLVM=1` mode, so
`CROSS_COMPILE` must point at a *real, existing* compiler prefix, not a
target-triple string. If your shell still has `CROSS_COMPILE` set from
building OpenSBI, it needs to be overridden here — `riscv-none-elf-`
(the real bare-metal toolchain from `riscv_toolchain_setup.md`), not
`riscv32-unknown-elf-` (which isn't a real binary and only worked for
OpenSBI's `LLVM=1` mode):

```bash
git clone --depth 1 https://github.com/torvalds/linux ~/opt/linux
cd ~/opt/linux
export ARCH=riscv
export CROSS_COMPILE=riscv-none-elf-
./scripts/kconfig/merge_config.sh arch/riscv/configs/defconfig arch/riscv/configs/32-bit.config
make -j$(nproc)
```

**If `merge_config.sh` prints warnings that `CONFIG_ARCH_RV32I`,
`CONFIG_32BIT`, or `CONFIG_NONPORTABLE` are "not in final .config"**
(not just "differs" — genuinely absent), the merge didn't actually
produce a 32-bit config, and the build would target 64-bit instead.
Real cause, confirmed directly from the kernel's own
[`arch/riscv/Kconfig`](https://github.com/torvalds/linux/blob/master/arch/riscv/Kconfig):
`ARCH_RV32I` is one option in a mutually-exclusive `choice` block
("Base ISA"), defaulting to `ARCH_RV64I` — `defconfig` already selects
`ARCH_RV64I=y`, and `32-bit.config`'s override doesn't reliably clear
that conflicting choice through a plain merge. Fix: edit `.config`
directly after the merge, then let Kconfig resolve the rest:

```bash
sed -i 's/CONFIG_ARCH_RV64I=y/# CONFIG_ARCH_RV64I is not set/' .config
sed -i 's/# CONFIG_ARCH_RV32I is not set/CONFIG_ARCH_RV32I=y/' .config
make olddefconfig
```

Then confirm it actually took before building:

```bash
grep -E "CONFIG_ARCH_RV32I|CONFIG_32BIT" .config
```

Output: `arch/riscv/boot/Image` — the raw, decompressed kernel image
`load_kernel` reads (not `vmlinux`, the unstripped ELF debug build).
See `docs/dev/multi_image_loader.md`'s Image-header section for what
this file's own 64-byte header describes.

## 3. Generate the device tree blob

Since this project deliberately matches QEMU's `virt` machine
addresses (see `docs/dev/peripherals_specs.md`, `peripherals_no_spec.md`),
the simplest and most reliable DTB is QEMU's own real, generated one —
not a hand-written `.dts`:

```bash
qemu-system-riscv32 -M virt -machine dumpdtb=virt.dtb -nographic -bios none
```

(This one has no environment variables to set, so it works the same
whether run inside the toolbox or, if installed via `rpm-ostree`
instead, directly on the host in Nushell.)

Output: `virt.dtb`, already in the binary flattened-tree form
`load_dtb` reads directly — `dumpdtb` outputs the compiled `.dtb`
format, so no separate `dtc` compilation step is needed.

## Result

Three files exist:

- `~/opt/opensbi/build/platform/generic/firmware/fw_dynamic.elf`
- `~/opt/linux/arch/riscv/boot/Image`
- `virt.dtb`

Update `loader.rs`'s `sbi_path`/`kernel_path`/`dtb_location` placeholder
strings in `boot_kernel` to point at these real paths, then run
`cargo run --bin run_os`.
