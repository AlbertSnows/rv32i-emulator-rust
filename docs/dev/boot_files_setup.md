# Boot files setup: OpenSBI, Linux kernel, device tree

How to build the three files `loader.rs`'s `boot_kernel` needs: an
OpenSBI firmware image, a Linux kernel `Image`, and a device tree blob
(DTB). See `docs/dev/multi_image_loader.md` for what each one is for
and how the handoff between them works.

This is the final, working recipe — every choice below (which kernel
tag, which config options, which DTB edits) exists because an earlier
attempt hit a real, specific failure. Where that's true, the failure
is described so a future change doesn't accidentally reintroduce it.

## Why these aren't committed to the repo

All three are large, fully reproducible build artifacts, not hand
-written source — same reasoning as the RISC-V toolchain and
sail-riscv already living outside the repo. Build output paths for
each should be added to `.gitignore` rather than committed.

## Prerequisites

- `~/opt/` as the install location, matching this project's existing
  convention for external tooling.
- A `toolbox` session for all the build steps below. **A `toolbox`
  session's default shell is bash, regardless of what your host's
  interactive shell is** (this project's host shell is Nushell — don't
  mix the two up: `export VAR=value` is a parser error in Nushell, and
  Nushell's `$env.VAR = value` is a parser error in bash). Every
  command below runs inside the toolbox, in bash:

  ```bash
  toolbox create riscv-build   # if you don't already have one
  toolbox enter riscv-build
  sudo dnf install -y clang lld llvm flex bison dtc qemu-system-riscv
  ```

  - `clang`/`lld`/`llvm` — OpenSBI's PIE requirement rules out the
    bare-metal GNU toolchain (see step 1); the kernel is also built
    with `LLVM=1` here for consistency and because it avoids toolchain
    -prefix mismatches between the two builds.
  - `flex`/`bison` — needed to build the kernel's own vendored `dtc`
    (`scripts/dtc`) as part of the kernel build itself.
  - `dtc` — the *host* device-tree-compiler package, for `fdtput`/
    `fdtdump`, used in step 3 to patch the generated DTB.
  - `qemu-system-riscv32` — to generate the real DTB, and optionally to
    cross-check emulator behavior against real hardware (see the last
    section).

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
time with `Your linker does not support creating PIEs`. `clang`/LLVM
is the fix; the same README confirms it can still produce PIE images
for a bare-metal target.

```bash
git clone https://github.com/riscv-software-src/opensbi ~/opt/opensbi
cd ~/opt/opensbi
export CROSS_COMPILE=riscv32-unknown-elf-
export PLATFORM_RISCV_XLEN=32
export PLATFORM_RISCV_ISA=rv32ima_zicsr_zifencei
make PLATFORM=generic FW_DYNAMIC=y LLVM=1
```

**`PLATFORM_RISCV_ISA` is narrowed on purpose.** Left unset, OpenSBI's
Makefile defaults to `rv$(XLEN)imafdc[_zicsr_zifencei]` — full
multiply/atomics/float/compressed. This emulator only implements the
base integer set plus `M` (multiply/divide) and `A` (atomics); it has
no compressed-instruction decoder and no floating-point unit. A
default-ISA OpenSBI build compiles compressed (2-byte) instructions
into the very first bytes of its entry point, which this emulator
can't decode at all — it isn't a bug to fix, it's a scope boundary:
either build the emulator's own decoder for the `C` extension (a large
feature, comparable in size to everything already built for the base
ISA), or don't emit those instructions in the first place. Narrowing
the ISA string is far cheaper and is what this project does.
`zicsr_zifencei` stays in the string because CSR instructions are used
everywhere in privileged code and modern toolchains require them
spelled out explicitly (they were split out of the base `I` extension
in a later revision of the ISA manual).

A few other things about this build that aren't obvious from the flags
alone, each a real error hit while developing this:

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
- **If you change `CROSS_COMPILE`/`PLATFORM_RISCV_ISA` between build
  attempts, run `make LLVM=1 clean` first.** `make` only tracks
  source-file timestamps, not which flags a previous build used —
  switching targets without cleaning first leaves stale `.o` files
  sitting in `build/`, which then fail to link against newly-compiled
  ones (`ld.lld: ... is incompatible with elf32lriscv`/`elf64lriscv`,
  depending on which direction the mismatch goes).

**One real upstream compile error will still occur**, unrelated to any
of the above: `lib/sbi/sbi_trap_ldst.c` fails to compile under `clang`
with `-Werror` when floating-point (`f`/`d`) is excluded from the ISA
string — `error: variable 'val' is used uninitialized whenever 'if'
condition is false [-Werror,-Wsometimes-uninitialized]`. This is a
real latent issue in OpenSBI's own floating-point trap-emulation code
path, only surfaced because F/D-disabled builds are less common than
the reverse. The fix, without touching OpenSBI's source (still real,
unmodified upstream code — just built with one warning downgraded):
compile that one file by hand with `-Wno-error=sometimes-uninitialized`
appended, then let `make` continue — it sees the fresh `.o` and moves
on. Get the *exact* compiler invocation `make` would have used via
`V=1`, so nothing else about the command line has to be reconstructed
by hand:

```bash
make PLATFORM=generic FW_DYNAMIC=y LLVM=1 V=1 2>&1 | grep "sbi_trap_ldst.c" | head -1
```

Take that full `clang ...` line, append
` -Wno-error=sometimes-uninitialized`, run it once, then re-run the
normal `make` command above — it picks up from there.

Output: `build/platform/generic/firmware/fw_dynamic.elf` — the exact
path `sbi_path` in `loader.rs` should point at.

## 2. Build the Linux kernel

Source: [`torvalds/linux`](https://github.com/torvalds/linux), **tag
`v6.1`** — not the tip of `master`. Mainline's RV32 support has a real,
reproducible bug: `arch/riscv/Kconfig` has `select HAVE_GENERIC_VDSO if
MMU && 64BIT`, but generic VDSO code still references
`__vdso_*_offset` symbols on some paths without properly gating them
for 32-bit — this reproduces on `--depth 1` master and on newer tags
alike, and manifests as undeclared-symbol build failures deep in
`arch/riscv/kernel/vdso/`. `v6.1` predates whatever introduced this and
builds cleanly.

Mainline also has no single "rv32 defconfig" —
[`arch/riscv/configs/`](https://github.com/torvalds/linux/tree/master/arch/riscv/configs)
ships a base `defconfig` plus a `32-bit.config` fragment, merged with
the kernel's own config-merging script:

```bash
git clone https://github.com/torvalds/linux ~/opt/linux
cd ~/opt/linux
git checkout v6.1
export ARCH=riscv
unset CROSS_COMPILE
./scripts/kconfig/merge_config.sh arch/riscv/configs/defconfig arch/riscv/configs/32-bit.config
```

**If `merge_config.sh` prints warnings that `CONFIG_ARCH_RV32I`,
`CONFIG_32BIT`, or `CONFIG_NONPORTABLE` are "not in final .config"**
(not just "differs" — genuinely absent), the merge didn't actually
produce a 32-bit config. Real cause, confirmed directly from the
kernel's own [`arch/riscv/Kconfig`](https://github.com/torvalds/linux/blob/master/arch/riscv/Kconfig):
`ARCH_RV32I` is one option in a mutually-exclusive `choice` block
("Base ISA"), defaulting to `ARCH_RV64I` — `defconfig` already selects
`ARCH_RV64I=y`, and `32-bit.config`'s override doesn't always clear
that conflicting choice through a plain merge (this didn't happen on
`v6.1` in practice, but did on a `--depth 1` master clone — worth
checking for regardless). If it happens, fix directly and re-derive:

```bash
sed -i 's/CONFIG_ARCH_RV64I=y/# CONFIG_ARCH_RV64I is not set/' .config
sed -i 's/# CONFIG_ARCH_RV32I is not set/CONFIG_ARCH_RV32I=y/' .config
make LLVM=1 olddefconfig
```

Confirm before continuing:

```bash
grep -E "CONFIG_ARCH_RV32I|CONFIG_32BIT" .config
```

### Narrowing the config to match this emulator

`defconfig` targets real hardware and QEMU's full feature set — most
of it is either unimplemented in this emulator (and will fault) or
genuinely unrelated (real storage/GPU/network hardware). Each of the
following was found by actually hitting the failure it describes, not
guessed in advance:

```bash
scripts/config --disable CONFIG_ATA
scripts/config --disable CONFIG_SCSI
scripts/config --disable CONFIG_DRM
scripts/config --disable CONFIG_FB
scripts/config --disable CONFIG_SOUND
scripts/config --disable CONFIG_USB
scripts/config --disable CONFIG_NETDEVICES
scripts/config --disable CONFIG_WLAN
scripts/config --disable CONFIG_EFI
scripts/config --disable CONFIG_RISCV_ISA_C
scripts/config --disable CONFIG_FPU
scripts/config --disable CONFIG_PCI
scripts/config --disable CONFIG_VIRTIO_MMIO
scripts/config --disable CONFIG_RTC_CLASS
make LLVM=1 olddefconfig
```

Why each group:

- **`ATA`, `SCSI`, `DRM`** — genuine 32-bit-arch compile bugs in these
  drivers under this specific (LLVM, RV32) toolchain combination, not
  anything to do with this emulator. `libahci.c` fails with
  `BUILD_BUG_ON failed: sizeof(_s) > sizeof(long)` (a struct-size
  assumption that only holds on 64-bit `long`); `nouveau` fails
  similarly. Since none of this hardware exists in this emulator
  anyway (no SATA, no GPU), disabling is strictly correct, not just
  a workaround.
- **`FB`, `SOUND`, `USB`, `NETDEVICES`, `WLAN`** — same reasoning:
  real-hardware driver categories this emulator has no device for.
  Disabling them also shrinks the build enormously.
- **`EFI`** — real QEMU boots this way but this project's `loader.rs`
  loads OpenSBI/kernel directly (see `multi_image_loader.md`), so no
  EFI runtime is needed. More importantly, `EFI`'s Kconfig entry
  unconditionally `select`s `RISCV_ISA_C` — Kconfig `select` cannot be
  overridden by disabling the target directly; you have to disable
  whatever *selects* it. This was found the hard way: disabling
  `RISCV_ISA_C` alone silently gets reverted back to `y` on the next
  `olddefconfig` until `EFI` is also gone.
- **`RISCV_ISA_C`, `FPU`** — matches the same ISA narrowing applied to
  OpenSBI above (see that section for the full "why"): this emulator
  has no compressed-instruction decoder and no FPU. Left enabled, the
  compiler emits instructions the kernel will crash on the moment it
  starts executing.
- **`PCI`, `VIRTIO_MMIO`, `RTC_CLASS`** — these drivers actively *probe
  real memory-mapped registers* during kernel init (PCI ECAM at
  `0x30000000`, virtio-mmio slots from `0x10001000`, the goldfish RTC
  at `0x00101000`) — addresses this emulator has never implemented a
  peripheral for. An unhandled fault during an init call is fatal to
  `kernel_init` itself (`PID 1`), which the kernel reports as
  `Attempted to kill init!` — a confusing message for what's really
  "this driver tried to read hardware that isn't there." `RTC_CLASS`
  specifically can't be disabled directly either: `arch/riscv/Kconfig.socs`'s
  `SOC_VIRT` (needed — it's the QEMU virt platform option itself)
  unconditionally does `select RTC_DRV_GOLDFISH if RTC_CLASS`, so
  `RTC_CLASS` itself has to go, the same `select`-can't-be-overridden
  pattern as `EFI` above.

Then build:

```bash
make LLVM=1 -j$(nproc)
```

Output: `arch/riscv/boot/Image` — the raw, decompressed kernel image
`load_kernel` reads (not `vmlinux`, the unstripped ELF debug build).
See `docs/dev/multi_image_loader.md`'s Image-header section for what
this file's own 64-byte header describes.

## 3. Generate and narrow the device tree blob

Since this project deliberately matches QEMU's `virt` machine
addresses (see `docs/dev/peripherals_specs.md`, `peripherals_no_spec.md`),
the simplest and most reliable DTB is QEMU's own real, generated one —
not a hand-written `.dts`:

```bash
qemu-system-riscv32 -M virt -machine dumpdtb=~/opt/virt.dtb -nographic -bios none
```

**This DTB needs two edits before use, both for the same underlying
reason: it accurately describes real QEMU's CPU, which has far more
capability than this emulator does.**

1. **Add a kernel command line requesting an early console.** Without
   one, kernel panics that happen before the normal console driver
   registers produce *no visible output at all* — messages get
   buffered internally and never reach the UART. `earlycon` bypasses
   normal driver probing and writes directly to the UART's known MMIO
   registers from the earliest possible point, which is how every real
   error message described in this document was actually captured.

   ```bash
   fdtput -t s ~/opt/virt.dtb /chosen bootargs "earlycon=uart8250,mmio,0x10000000,9600n8 console=ttyS0"
   ```

2. **Narrow `riscv,isa-extensions`/`riscv,isa` on the CPU node to match
   this emulator.** Real QEMU's DTB lists dozens of extensions
   (`f`,`d`,`c`,`h`, `zicbom`, `zicboz`, `sstc`, `svadu`, and more) that
   this emulator doesn't implement. OpenSBI and the kernel both trust
   this property and will try to *use* whatever it claims is present —
   most visibly, `sstc` (a newer extension letting supervisor mode set
   its own timer via the `stimecmp` CSR directly, instead of an SBI
   call) causes an illegal-instruction trap the moment the kernel tries
   it, which OpenSBI's own software-emulation fallback also can't
   fully recover from. The fix is to make the DTB honest about what
   this emulator actually supports:

   ```bash
   fdtput -t s ~/opt/virt.dtb /cpus/cpu@0 riscv,isa-extensions i m a zicsr zifencei
   fdtput -t s ~/opt/virt.dtb /cpus/cpu@0 riscv,isa "rv32ima_zicsr_zifencei"
   ```

Output: `~/opt/virt.dtb`, already in the binary flattened-tree form
`load_dtb` reads directly — `dumpdtb` outputs the compiled `.dtb`
format, so no separate `dtc` compilation step is needed.

## Result

Three files exist:

- `~/opt/opensbi/build/platform/generic/firmware/fw_dynamic.elf`
- `~/opt/linux/arch/riscv/boot/Image`
- `~/opt/virt.dtb`

Update `loader.rs`'s `sbi_path`/`kernel_path`/`dtb_location` strings in
`boot_kernel` to point at these paths, then run `cargo run --bin
run_os` (or the `--release` build — running unoptimized instruction
-by-instruction interpretation of a real Linux boot is dramatically
slower than release mode, on the order of tens of minutes vs seconds).

## Cross-checking against real QEMU

When a boot failure's cause isn't obvious from this emulator's own
state alone, the fastest way to tell "is this a bug in the emulator"
from "is this a bug in how the boot files were built" is to boot the
*exact same* three files on real QEMU:

```bash
qemu-system-riscv32 -M virt -nographic \
  -bios ~/opt/opensbi/build/platform/generic/firmware/fw_dynamic.bin \
  -kernel ~/opt/linux/arch/riscv/boot/Image \
  -dtb ~/opt/virt.dtb \
  -append "earlycon=uart8250,mmio,0x10000000,9600n8 console=ttyS0" \
  -m 128M
```

(Note: `-bios` wants the `.bin` here, not the `.elf` — QEMU's `-bios`
loader doesn't apply PIE relocation the way this emulator's `load_elf`
does, so pointing it at the raw ELF produces a "ROM regions
overlapping" error. OpenSBI's build also produces this `.bin` variant
alongside the `.elf`, in the same output directory.)

If real QEMU reaches the same point and hits the same problem, the
bug is in the boot files or their configuration — look there. If real
QEMU works fine, the bug is in the emulator. Going one step further,
GDB can attach to a *paused* real QEMU instance (`-s -S` instead of
running normally) and break at a specific instruction address to
inspect exactly what register/CSR state real hardware has at that
point, to compare directly against what this emulator computes for
the same address — this is how the kernel/DTB placement bugs below
were actually found, rather than guessed at from disassembly alone:

```bash
qemu-system-riscv32 -M virt -nographic -bios ... -kernel ... -dtb ... -m 128M -s -S &
riscv-none-elf-gdb -q -batch -ex "target remote localhost:1234" \
  -ex "break *0xADDRESS" -ex "continue" -ex "print/x \$a0" -ex "print/x \$satp"
```

With `-S`, QEMU starts paused at reset (M-mode, no paging yet), so a
physical address like `0x80400000` can be read directly via
`x/4xb 0x80400000` without any translation — useful for confirming
exactly where QEMU itself placed something in memory, independent of
whatever this project's own loader assumes.
