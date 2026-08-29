# RISC-V Bare-Metal Toolchain Setup

How to install a RISC-V cross-compiler capable of building `riscv-tests`
(and other bare-metal RV32/RV64 assembly/C) on Linux. 

## Why this specific toolchain

You need a **bare-metal** (`elf`/newlib) RISC-V toolchain, not a
**hosted** (`linux-gnu`) one. `riscv-tests` binaries have no OS under
them: no libc, no kernel, they define their own `_start`, install their
own trap vector, and link against a custom linker script. 

This guide uses [xPack's `riscv-none-elf-gcc`](https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack),
which ships prebuilt bare-metal binaries. 

## Prerequisites

- `curl`, `tar`, `sha256sum` (standard on most Linux distros)
- `~/opt/` as a home for locally-installed tools that don't belong under
  system package management. This matters especially on an
  rpm-ostree/atomic system (like Fedora Silverblue/Kinoite) where `/usr`
  isn't meant to be hand-installed into — but it's a reasonable default
  anywhere.

## Steps

### 1. Download the tarball and its checksum

```bash
mkdir -p ~/opt && cd ~/opt
curl -L -O https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/<TAG>/xpack-riscv-none-elf-gcc-<VERSION>-linux-x64.tar.gz
curl -L -O https://github.com/xpack-dev-tools/riscv-none-elf-gcc-xpack/releases/download/<TAG>/xpack-riscv-none-elf-gcc-<VERSION>-linux-x64.tar.gz.sha
```

### 2. Extract

```bash
tar xzf xpack-riscv-none-elf-gcc-<VERSION>-linux-x64.tar.gz
```

This produces `~/opt/xpack-riscv-none-elf-gcc-<VERSION>/`.

### 3. Sanity-check the binary actually runs

```bash
~/opt/xpack-riscv-none-elf-gcc-<VERSION>/bin/riscv-none-elf-gcc --version
```

Expect something like:

```
riscv-none-elf-gcc (xPack GNU RISC-V Embedded GCC x86_64) <VERSION>
```

A successful download and extraction doesn't guarantee the binary
actually executes on your system — check this before moving on.

### 6. Add it to your `PATH`

Edit `~/.bashrc` (or the equivalent for your shell — check `$SHELL`
first if unsure), adding near your other `PATH` exports:

```bash
# riscv-none-elf-gcc (bare-metal RISC-V toolchain, for building riscv-tests)
export PATH="$HOME/opt/xpack-riscv-none-elf-gcc-<VERSION>/bin:$PATH"
```

### 7. Verify in a new shell

Confirm the change actually takes effect:

```bash
bash -lc 'which riscv-none-elf-gcc && riscv-none-elf-gcc --version'
```

## Result

`riscv-none-elf-gcc`, `riscv-none-elf-readelf`, `riscv-none-elf-objdump`,
and the rest of the toolchain's binaries are now on `PATH`. 