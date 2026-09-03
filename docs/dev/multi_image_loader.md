# Multi-image loader: OpenSBI + Linux kernel + device tree

## Why

The existing ELF loader (used for riscv-tests/riscv-arch-test) loads exactly
one program, always at `BASE_ADDRESS`, and the CPU just starts executing at
that one address with everything else zeroed. Booting Linux is a different
shape of problem entirely: three separate binaries need to be in memory
*simultaneously*, each at its own address, and the CPU's very first
instructions need specific register values already in place — not zeros —
for the handoff between them to make any sense at all. None of that is
something the current loader does, or was ever designed to do.

## The core problem, conceptually

Think about what the CPU actually knows at the moment it starts running:
nothing. No operating system context, no idea what hardware exists, no idea
where in memory anything useful is. Real hardware solves this by having each
stage of boot tell the *next* stage just enough to get started, via a small,
fixed set of CPU registers — a handoff, not unlike handing a new employee a
single sticky note with "your desk is here, your manager is there" instead of
expecting them to figure out the whole org chart on day one.

For RISC-V, that handoff happens in stages:

1. **Firmware (OpenSBI) starts first, in M-mode** — the most-trusted level.
   Its job is to set up the low-level privileged plumbing (trap delegation,
   the SBI call surface covered in `docs/dev/peripherals_specs.md`) so that
   everything running beneath it doesn't have to touch that machinery
   directly.
2. **OpenSBI then hands off to the kernel, in S-mode** — a *drop* in privilege,
   deliberately: the OS doesn't need, and shouldn't have, M-mode's level of
   trust.
3. **The kernel uses the device tree to discover what hardware it's running
   on** — instead of every kernel build needing to hardcode a machine's exact
   memory map, it reads a data structure describing it (this is exactly why
   the DTB has to already be in memory and its address known *before* the
   kernel's very first instruction runs — the kernel has no other way to find
   out where anything is).

So the three files aren't an arbitrary requirement — each one has a distinct
job (privileged services broker, actual OS, hardware description), and each
needs to already exist in memory, correctly, before the one after it can make
sense of anything. The "handoff protocol" below is just the answer to: *how
does each stage tell the next one enough to get started?*

## What has to be loaded

1. **OpenSBI firmware** (`fw_jump.bin` or `fw_dynamic.bin`, or their `.elf`
   equivalents) — runs first, in M-mode. This is the piece that turns "an
   emulator that can execute privileged instructions" into "a machine Linux
   will actually boot on" (see `docs/plans/plans.md`'s BIOS section).
2. **Linux kernel `Image`** — the raw, decompressed RISC-V kernel image (not
   `vmlinux`, which is an unstripped ELF debug build meant for debuggers, not
   for booting).
3. **Device tree blob (DTB)** — describes the machine to the kernel (see
   `docs/dev/peripherals_specs.md`'s Device Tree section for how to generate
   one that matches this project's peripheral addresses).

## The OpenSBI handoff protocol

Straight from OpenSBI's own docs
([`docs/firmware/fw.md`](https://github.com/riscv-software-src/opensbi/blob/master/docs/firmware/fw.md)):

> The previous booting stage will pass information via the following
> registers of RISC-V CPU:
> * hartid via *a0* register
> * device tree blob address in memory via *a1* register. The address must
>   be aligned to 8 bytes.

This is the entire base contract, and it's deliberately tiny: two registers,
two facts ("who am I," "where's the hardware description"). Every OpenSBI
firmware variant honors this same base contract — where they differ is
purely in how *OpenSBI itself* learns where the kernel is, since that's
information *this loader* has to supply, not something OpenSBI can know on
its own. Three variants exist:

- **`FW_JUMP`** ([`docs/firmware/fw_jump.md`](https://github.com/riscv-software-src/opensbi/blob/master/docs/firmware/fw_jump.md)) — the kernel's entry
  address is a **compile-time constant** (`FW_JUMP_ADDR`), baked into the
  firmware binary when OpenSBI itself was built.
- **`FW_DYNAMIC`** ([`docs/firmware/fw_dynamic.md`](https://github.com/riscv-software-src/opensbi/blob/master/docs/firmware/fw_dynamic.md)) — the kernel's
  entry address is supplied **at runtime**, by the loader, via a small struct
  in memory.
- **`FW_PAYLOAD`** — the kernel is embedded directly inside the OpenSBI
  binary at build time. Not applicable here, since this project loads the
  kernel as a separate file, not something baked into a custom OpenSBI build.

**Why `FW_DYNAMIC`, specifically, and not `FW_JUMP`:** walk through what
`FW_JUMP` would actually require. A prebuilt `fw_jump.bin` (say, downloaded
rather than self-compiled) has `FW_JUMP_ADDR` frozen into it at the exact
address whoever built it assumed the kernel would live at. This loader's
kernel placement is computed dynamically (see `riscv_calc_kernel_start_addr`
below — it depends on exactly how big OpenSBI itself turned out to be, which
varies by build). If those two addresses don't match *exactly*, OpenSBI jumps
confidently to the wrong address and the kernel never runs — a silent,
confusing failure mode with no error message, since OpenSBI has no way to
know the address it was told is wrong. `FW_DYNAMIC` sidesteps this
completely: the loader computes the real kernel address at load time and
just tells OpenSBI directly, so there's no fixed constant that has to happen
to agree with anything else.

### `FW_DYNAMIC`'s handoff struct, exact layout

From OpenSBI's own header,
[`include/sbi/fw_dynamic.h`](https://github.com/riscv-software-src/opensbi/blob/master/include/sbi/fw_dynamic.h):

```c
struct fw_dynamic_info {
    unsigned long magic;      // must be 0x4942534f ("OSBI" as ASCII, hex)
    unsigned long version;    // 0x2 (FW_DYNAMIC_INFO_VERSION_MAX)
    unsigned long next_addr;  // kernel's entry address
    unsigned long next_mode;  // 0x1 = S-mode (FW_DYNAMIC_INFO_NEXT_MODE_S)
    unsigned long options;    // 0 (no special OpenSBI library options)
    unsigned long boot_hart;  // -1UL to disable "preferred boot hart"; 0 is also fine for a single-hart target
} __packed;
```

The `magic` field is a self-check: OpenSBI reads it back and refuses to
trust the rest of the struct unless it matches exactly, which is a sane
defense against "the loader forgot to set this struct up at all and OpenSBI
is reading uninitialized memory as if it were real handoff data." `version`
exists for forward-compatibility — different OpenSBI releases can extend
this struct, and `version` tells OpenSBI which fields are actually present.
`next_mode = S` is the explicit instruction "drop to Supervisor mode before
jumping to `next_addr`" — this is the literal mechanism behind the privilege
drop described above; without it, nothing would tell OpenSBI it should
change privilege levels at all before jumping to the kernel.

`unsigned long` is 4 bytes on RV32, 8 on RV64 — this project's fields are
all 4 bytes wide. The struct's address is passed in **`a2`** — a third
register, beyond the `a0`/`a1` base contract, specific to `FW_DYNAMIC`.
That's not part of any universal RISC-V convention; it's OpenSBI's own
addition, confirmed by QEMU's own comment on the matter (see below):
"doesn't break any other firmware as long as they don't expect any certain
value in `a2`" — i.e., other firmware types simply ignore `a2`, so setting
it is safe regardless of which firmware type ends up loaded.

## How QEMU (the real reference implementation) actually does all of this

QEMU's `virt` machine performs exactly this handoff, in
[`hw/riscv/boot.c`](https://github.com/qemu/qemu/blob/master/hw/riscv/boot.c) — worth reading directly rather than reinventing, since
this project already matches QEMU's `virt` addresses/layout elsewhere, and
because seeing the *actual*, currently-shipping implementation is a stronger
guarantee of correctness than re-deriving the mechanism from documentation
prose alone (docs can drift out of date with the code; the code that a real,
widely-used emulator ships is what actually has to work).

- **`riscv_setup_rom_reset_vec`** (line 483) — QEMU doesn't jump the hart
  straight to OpenSBI's entry point. At power-on, the hart's PC starts at a
  fixed hardware reset address, and QEMU places a *tiny hand-assembled boot
  ROM* there — six real RISC-V instructions, not C code — whose entire job is
  computing the three handoff registers and then jumping. In order:
  1. `auipc t0, ...; addi a2, t0, ...` — compute and load `a2` = address of
     the `fw_dynamic_info` struct, which QEMU places in this same tiny ROM,
     right after the six instructions.
  2. `csrr a0, mhartid` — `a0` = the *real* hart ID, read live from the
     `mhartid` CSR (this project already has `MHARTID` defined at `0xF14`)
     rather than a hardcoded constant — necessary on real hardware because a
     multi-hart machine needs each hart to discover its *own* distinct ID by
     reading this register itself; there's no other way for a hart to learn
     which one it is.
  3. `lw a1, 32(t0)` — `a1` = the DTB address, loaded from a data word stored
     later in this same ROM (not computed by the CPU — pre-baked in by QEMU
     before the hart ever starts).
  4. `lw t0, 24(t0); jr t0` — load OpenSBI's real entry address from another
     pre-baked data word, then jump to it. This is the actual moment
     execution leaves the tiny reset ROM and firmware takes over.
  5. The `.data` half of this ROM (offsets 24 and 32, measured from the
     `auipc` instruction's own address) stores exactly two words: OpenSBI's
     entry address, then the DTB's address — the two numbers instructions 3
     and 4 above load.

  Why go through a hand-assembled ROM at all, instead of just starting the
  hart at OpenSBI directly? Two reasons specific to QEMU as a general-purpose
  emulator that this project doesn't share: it needs to work for *any*
  number of harts (each one independently reads its own `mhartid`, which only
  makes sense as executed code, not as a value the emulator could just poke
  into a register once), and it needs to support relocatable ROMs at various
  board-specific reset addresses. This project has neither constraint — a
  single, fixed hart, with a loader that already knows every address at load
  time — so it can skip the ROM entirely and just write the four handoff
  registers directly, as covered in "What's needed" below.

- **`riscv_rom_copy_firmware_info`** (line 422) — the function that actually
  populates the `fw_dynamic_info` struct's real bytes (the assembly above
  only reads two pre-existing words; it doesn't build the struct). Confirmed
  field values, read directly from current QEMU source: `magic`/`version` as
  defined above, `next_mode = S` (`FW_DYNAMIC_INFO_NEXT_MODE_S`),
  `next_addr = kernel_entry` (the real Linux entry address, computed
  elsewhere and passed in as a parameter), `options = 0`, `boot_hart = 0`.
  This confirms the values documented in OpenSBI's own header are exactly
  what a real, shipping implementation actually puts there — not merely a
  theoretical default someone wrote in a comment once.

- **`riscv_calc_kernel_start_addr`** (line 94) — where the kernel actually
  gets placed: `align_up(firmware_end_addr, 4 * MiB)` for RV32 (`2 * MiB` for
  RV64) — i.e., right after wherever OpenSBI happens to end, rounded up to a
  4MiB boundary. This is *why* `FW_JUMP`'s fixed compile-time address is
  fragile in practice: this formula's result depends on OpenSBI's own build
  size, which can change between OpenSBI versions or build configurations —
  there is no single "correct" fixed address that stays right forever.

- **`riscv_compute_fdt_addr`** (line 350) — the DTB is placed *after* the
  kernel (or after the initrd, if one was loaded) — again, computed from
  where the kernel actually ended up, not a fixed offset from `BASE_ADDRESS`.

## The Linux kernel's own placement hint: the Image header

Every decompressed RISC-V kernel `Image` starts with a documented 64-byte
header — [Linux `Documentation/arch/riscv/boot-image-header.rst`](https://github.com/torvalds/linux/blob/master/Documentation/arch/riscv/boot-image-header.rst):

```c
u32 code0;                  // executable code
u32 code1;                  // executable code
u64 text_offset;             // image load offset, little endian
u64 image_size;              // effective image size, little endian
u64 flags;                   // kernel flags, little endian
u32 version;
u32 res1 = 0;
u64 res2 = 0;
u64 magic = 0x5643534952;    // "RISCV", little endian
u32 magic2 = 0x05435352;     // "RSC\x05", little endian
u32 res3;
```

This header exists so a loader never has to guess or hardcode anything about
a specific kernel build — it's self-describing. `image_size` is mandatory —
"Booting will fail otherwise" per the doc — the loader needs it to know
exactly how many bytes to actually copy into memory (get this wrong and
you'll either truncate the kernel or read garbage past the end of the file).
`text_offset` tells the loader the kernel's *preferred* offset from wherever
it's placed; combined with `riscv_calc_kernel_start_addr`'s alignment
formula above, this is how a real loader arrives at the kernel's final load
address and, from that, its entry point (`code0`/`code1` at the very start
of the image are real executable instructions — the kernel's actual entry
point is the load address itself, not a separate field to look up).

## Putting the whole flow together, start to finish

With all the pieces above named, here's the complete sequence, in order,
tying the "why" back to the mechanism:

1. Loader places OpenSBI in memory, places the kernel after it (using the
   Image header to know how much space it needs and where it wants to sit),
   places the DTB after that, and builds the `fw_dynamic_info` struct
   somewhere in memory too.
2. Loader sets `pc` = OpenSBI's entry, `a0` = hart ID, `a1` = DTB address,
   `a2` = `fw_dynamic_info` struct address — the entire handoff, done.
3. Execution starts. OpenSBI (M-mode) reads `a2`, checks the `magic` field,
   and now knows exactly where the kernel is and what mode to jump to it in.
4. OpenSBI finishes its own M-mode setup (trap delegation, SBI call
   handlers — see `docs/plans/plans.md`'s BIOS section) and jumps to
   `next_addr`, dropping privilege to S-mode per `next_mode`, handing the
   *same* `a1` (DTB address) forward to the kernel — the kernel never talks
   to the loader directly, only to whatever OpenSBI forwards.
5. The kernel (S-mode) reads the DTB at the address it was handed, discovers
   the machine's peripherals (UART, PLIC, memory layout — everything from
   `docs/dev/peripherals_specs.md` and `peripherals_no_spec.md`), and boots
   normally from there.

## What's needed for this project

1. A loader function that takes three file paths (or byte buffers) — OpenSBI
   image, Linux `Image`, DTB — and:
   - Loads OpenSBI at `BASE_ADDRESS` (`0x80000000`, matching this project's
     existing convention and QEMU's `VIRT_DRAM` base).
   - Reads the kernel `Image` header (parses `text_offset`/`image_size`),
     places the kernel at `align_up(opensbi_end, 4 * MiB)`.
   - Places the DTB after the kernel.
   - Builds a `fw_dynamic_info` struct (4-byte fields, matching RV32) in
     memory, with `next_addr` = the kernel's real entry address, `next_mode`
     = S (`1`), and the rest matching the values above.
2. CPU initial state before the first `step()`: `pc` = OpenSBI's entry
   address, `a0` = hart ID (`0`, single hart), `a1` = the DTB's address,
   `a2` = the `fw_dynamic_info` struct's address. As explained above, this
   project doesn't need QEMU's tiny reset-vector ROM trick at all — that
   exists purely to support multiple harts and relocatable reset addresses,
   neither of which apply here — so the loader can just write these four
   registers directly, once, rather than emitting real instructions to
   compute them at boot time.
