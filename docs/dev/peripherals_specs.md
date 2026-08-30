# Peripheral specs: PLIC, SBI, virtio-mmio, Device Tree

Four peripherals/interfaces needed for the peripherals -> BIOS -> OS work
have real, ratified specification documents — unlike CLINT and the UART,
which are de facto conventions with no RISC-V-specific spec at all (see
`docs/dev/peripherals_no_spec.md`). All addresses below match QEMU's
`virt` machine (`hw/riscv/virt.c`, `include/hw/riscv/virt.h`), the same
target this emulator's existing CLINT implementation already matches —
worth continuing deliberately, since it means an existing Linux
defconfig and device tree already work without inventing a bespoke
machine.

## PLIC — Platform-Level Interrupt Controller

**What it is.** A memory-mapped device that multiplexes many external
interrupt sources (UART RX, virtio devices, etc.) onto each hart's
single `meip`/`seip` line, with per-source priority and per-context
enable/threshold/claim registers.

**Why this project needs it.** `mip.MEIP`/`SEIP` (already modeled in
`csr.rs`) are the *destination* of an external interrupt notification,
but this emulator has no mechanism to *set* them from a device. PLIC is
that mechanism — without it, any peripheral beyond a polled UART can
never actually interrupt the CPU.

**Spec:** [RISC-V PLIC Specification v1.0.0](https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf)
(ratified 2023-03, CC-BY-4.0, 16 pages — short and precise).

### Section outline

- **§1 Introduction** — the model: gateways convert raw device signals
  into a single pending request per source; the PLIC core holds
  priority/pending/enable state; targets are hart+privilege contexts.
  Up to 1023 interrupt sources (ID 0 reserved = "no interrupt") and
  15872 contexts, though real implementations declare far fewer.
  - §1.1 Interrupt Targets and Hart Contexts — a "context" is (hart,
    privilege mode); this project needs exactly one context per
    implemented privilege mode per hart (M and S, single hart).
  - §1.2 Interrupt Gateways — level-triggered sources (UART fits this)
    won't raise a second request for the same source until the
    previous one is completed.
  - §1.4 Interrupt Identifiers — IDs start at 1; lower ID wins
    priority ties.
- **§2 Operation Parameters** — names the six register blocks: Priority,
  Pending, Enable, Threshold, Claim, Completion (claim and completion
  share one register).
- **§3 Memory Map** — the actual byte layout, all registers 32-bit,
  accessed with `lw`/`sw`:
  - `base + 0x000000`–`0x000FFC`: per-source priority (source 0
    reserved)
  - `base + 0x001000`–`0x00107C`: pending bits, 32 sources/word
  - `base + 0x002000` + `0x80*context`: per-context enable bits
  - `base + 0x200000` + `0x1000*context`: threshold (offset +0) and
    claim/complete (offset +4) per context
- **§4 Interrupt Priorities** — priority 0 = "never interrupt"; higher
  integer = higher priority; can legally be hardwired if not
  implementing WARL discovery.
- **§5 Interrupt Pending Bits** — read-only status; bit `(N mod 32)` of
  word `(N/32)`.
- **§6 Interrupt Enables** — one bit per (source, context); bit 0 of
  context 0's first word is hardwired 0 (source 0 doesn't exist).
- **§7 Priority Thresholds** — per-context WARL register; masks all
  interrupts at or below the threshold value.
- **§8 Interrupt Claim Process** — reading the claim/complete register
  atomically returns the highest-priority pending source ID for that
  context *and* clears its pending bit; returns 0 if nothing pending.
- **§9 Interrupt Completion** — writing the claimed ID back to the same
  register tells the gateway to accept a new request from that source.

### What's needed (minimal, single hart, few sources)

A source count in the single digits (UART = 1, plus a couple of
virtio-mmio lines if that comes later) means most of the 1023-source /
15872-context address space is unused/reserved space you don't need to
back with real storage — implement it as "reads as zero, ignore
out-of-range writes" and only give real backing to the sources and
contexts you declare. Two contexts (M and S) is enough for one hart.
Wire the PLIC's per-context EIP output into the existing
`mip.MEIP`/`SEIP` CSR bits so `step()`'s trap-check logic (whatever
already polls `mip`) picks it up unchanged.

## SBI — Supervisor Binary Interface

**Not something this project implements** — a real OpenSBI binary gets
loaded and run instead (see `docs/plans/plans.md`'s BIOS/firmware
section). This section is here for a narrower reason: OpenSBI running
on top of this emulator will constantly exercise this exact mechanism,
and understanding it is what lets you tell "our CPU emulation is
misbehaving" apart from "OpenSBI itself hit a real edge case" when
something goes wrong after handoff.

**What it is.** The ABI between M-mode firmware and S-mode software
(Linux). S-mode issues `ecall` with arguments in `a0`-`a7`; the
firmware's trap handler dispatches on extension ID (`a7`)/function ID
(`a6`) and returns `(error, value)` in `a0`/`a1`.

**Why it matters here even though we don't implement it.** Every SBI
call Linux makes traps into M-mode on *our* CPU — the `ecall` trap
itself, `mtvec`/`mepc`/`medeleg` handling, and `mret` back to S-mode
are all this project's CPU emulation (item #11, already done), not
OpenSBI's. If a boot hangs or misbehaves right after an SBI call, the
question is always "did our trap/CSR mechanics do the wrong thing" vs.
"did OpenSBI's own logic do the wrong thing given what we handed it" —
knowing the spec's shape is what lets you tell those apart instead of
guessing.

**Spec:** [RISC-V SBI Specification v3.0](https://github.com/riscv-non-isa/riscv-sbi-doc/releases/download/v3.0/riscv-sbi.pdf)
(111 pages, 23 chapters — most of it (PMU, CPPC, nested acceleration,
message proxy, ...) is out of scope for a single-hart Linux boot; the
chapters below are the ones that matter).

### Section outline (chapters actually needed)

- **§3 Binary Encoding** — the calling convention itself: `a7`=EID,
  `a6`=FID (SBI v0.2+ only), `a0`-`a5`=args, return `(a0=error,
  a1=value)`. Unsupported EID/FID must return `SBI_ERR_NOT_SUPPORTED`
  (-2), not trap. Table 1 lists all standard error codes.
- **§4 Base Extension (EID `0x10`)** — mandatory for every SBI
  implementation. §4.1-§4.7: `sbi_get_spec_version`,
  `sbi_get_impl_id`/`_version`, `sbi_probe_extension` (how Linux
  detects which of the extensions below exist), `sbi_get_mvendorid`/
  `marchid`/`mimpid` (can all legally return 0). This is the one
  extension with no error paths — it must always work.
- **§5 Legacy Extensions (EIDs `0x00`-`0x0F`)** — different calling
  convention (no FID, only `a0` returned). §5.1 `sbi_set_timer`, §5.2
  `sbi_console_putchar`, §5.3 `sbi_console_getchar`, §5.9
  `sbi_shutdown`. Deprecated in favor of the extensions below, but
  Linux still falls back to these when `sbi_get_spec_version` reports
  < 0.2 or the modern extensions probe as absent — **this is the
  smallest possible BIOS**: implement only Base + these four legacy
  calls and a single-hart Linux kernel can still boot, print to
  console, take timer ticks, and shut down cleanly.
- **§6 Timer Extension (EID `"TIME"`)** — modern replacement for
  `sbi_set_timer`; one function, same semantics.
- **§9 Hart State Management (EID `"HSM"`)** — needed for multi-hart
  boot (starting/stopping secondary harts) and Linux's hart-status
  queries. Table 17 lists the state machine (STARTED/STOPPED/
  START_PENDING/etc). Skippable for a genuinely single-hart target if
  Linux is configured/patched not to probe it, but real-world defconfig
  kernels do probe it — check before assuming it's optional.
- **§10 System Reset (EID `"SRST"`)** — modern replacement for
  `sbi_shutdown`; `sbi_system_reset(reset_type, reset_reason)`, where
  `reset_type` 0/1/2 = shutdown/cold reboot/warm reboot (Table 26).
- **§12 Debug Console (EID `"DBCN"`)** — modern replacement for legacy
  console putchar/getchar, buffer-based instead of byte-at-a-time;
  recent Linux prefers this if available but falls back to legacy.

### What OpenSBI will actually do on this emulator

Real OpenSBI (github.com/riscv-software-src/opensbi) implements every
chapter above already — Base, Legacy, TIME, HSM, SRST, DBCN — so there
is no "which subset to build" decision to make; it's all there,
unconditionally. The only real question when tracing a boot problem is
*which of these chapters is the failing call in*, so you know where to
look next:

- A hang or wrong behavior on the very first SBI call after boot ->
  almost certainly §3/§4 (Base) or the `ecall`/`mtvec` trap path itself
  — check our CPU's trap mechanics first.
- Console output never appears -> §5 (`sbi_console_putchar`) or §12
  (DBCN) — check whether OpenSBI's own UART driver is reading the
  right MMIO addresses for the peripheral we built, per
  `docs/dev/peripherals_no_spec.md`.
- Boot stalls waiting on other harts -> §9 (HSM) — expected on a
  single-hart target only if OpenSBI/Linux are configured for exactly
  one hart; otherwise it'll wait for a hart-start that will never come.
- Reboot/poweroff does nothing -> §10 (SRST).

Reading OpenSBI's own source for whichever chapter is implicated (its
platform code lives under `platform/generic/`, matching the `generic`
platform mentioned above) is the actual next step once one of these is
suspected — not this spec document, which only tells you what OpenSBI
*should* do, not what its code on this specific call path *actually*
does.

## virtio-mmio (only needed for a real disk/root filesystem)

**What it is.** A device discovery/transport convention for virtio
devices without PCI — a small set of memory-mapped control registers
per device, feature negotiation, and virtqueues (ring buffers) for
actual I/O.

**Why this project needs it** — conditionally. An initramfs baked
into the kernel image needs no disk device at all; virtio-mmio only
matters if booting from a real, separate root filesystem is a goal.
Defer this until the UART/PLIC/SBI path is proven.

**Spec:** [OASIS Virtio v1.2](https://docs.oasis-open.org/virtio/virtio/v1.2/csd01/virtio-v1.2-csd01.html), §4.2 (transport) and §5.2 (block device).

### Section outline

- **§4.2 Virtio Over MMIO** — "Virtual environments without PCI
  support... might use simple memory mapped device ('virtio-mmio')...
  most operations including device initialization, queue configuration
  and buffer transfers are nearly identical [to PCI virtio]."
  - §4.2.1 MMIO Device Discovery — no generic discovery; the guest OS
    is simply told the register base and IRQ, conventionally via
    device tree (`compatible = "virtio,mmio"`, `reg`, `interrupts`).
  - §4.2.2 MMIO Device Register Layout (Table 4.1) — the actual
    registers, all little-endian 32-bit: `MagicValue` (0x000, must read
    `0x74726976`, "virt" in ASCII), `Version` (0x004, `0x2` for modern),
    `DeviceID` (0x008, `2` = block device per §5 Device Types),
    `VendorID` (0x00c), `DeviceFeatures`/`DeviceFeaturesSel` (0x010/
    0x014), `DriverFeatures`/`DriverFeaturesSel` (0x020/0x024),
    `QueueSel` (0x030), `QueueNumMax`/`QueueNum` (0x034/0x038),
    `QueueReady` (0x044), `QueueNotify` (0x050), `InterruptStatus`/
    `InterruptACK` (0x060/0x064), `Status` (0x070, write 0 to reset),
    `QueueDescLow`/`High`, `QueueDriverLow`/`High`, `QueueDeviceLow`/
    `High` (0x080-0x0a4, the three virtqueue ring addresses).
  - §4.2.3 MMIO-specific Initialization and Device Operation —
    §4.2.3.1 Device Initialization is the exact register-write sequence
    a driver performs (status flags in a specific order, feature
    negotiation, then per-queue setup).
  - §4.2.4 Legacy interface — an older, incompatible register layout
    (`Version` = `0x1`); ignore unless supporting very old guests.
- **§5.2 Block Device** — the device this project would actually
  implement:
  - §5.2.1 Device ID = `2`.
  - §5.2.3 Feature bits — `VIRTIO_BLK_F_RO` (5, read-only), `_BLK_SIZE`
    (6), `_FLUSH` (9); everything else (multiqueue, discard, secure
    erase, topology) is skippable for a minimal read/write disk.
  - §5.2.4 Device configuration layout — `struct virtio_blk_config`,
    starting with `le64 capacity` (device size in 512-byte sectors) —
    the only field that's unconditionally present.

### What's needed

One `DeviceID=2` block device is the realistic target. Skip every
optional feature bit; a device that only ever reports `capacity` and
handles plain read/write requests off a single virtqueue is a
complete, driver-compatible implementation. QEMU's `virt` machine
convention is a `0x1000`-byte MMIO window per device starting at
`0x10001000` (see `docs/dev/peripherals_no_spec.md`'s address table).

## Device Tree

**What it is.** A data structure describing the hardware to the kernel
at boot — CPUs, memory, and every peripheral's compatible-string,
register range, and interrupt wiring — passed as a flattened binary
blob (DTB) at a known physical address.

**Why this project needs it.** Linux's RISC-V boot path has no
platform-detection mechanism beyond the DTB it's handed; every
peripheral built above is invisible to the kernel until it's described
here. This is also the last piece that ties everything together —
it's written only once the actual memory map (UART/PLIC/virtio
addresses, IRQ numbers) is fixed.

**Spec:** [Devicetree Specification v0.4](https://github.com/devicetree-org/devicetree-specification/releases/download/v0.4/devicetree-specification-v0.4.pdf).

### Section outline (relevant chapters)

- **Ch.1 Introduction** — §1.3 32-bit and 64-bit Support: `#address-cells`/
  `#size-cells` determine how wide `reg`/`ranges` entries are; for this
  RV32 target both are `1`.
- **Ch.2 The Devicetree** — the structural chapters:
  - §2.2 Devicetree Structure and Conventions — node naming
    (`name@unit-address`), path names, generic property types.
  - §2.3 Standard Properties — §2.3.1 `compatible` (the string(s) the
    kernel driver-matches against, e.g. `"ns16550a"`, `"riscv,plic0"`,
    `"virtio,mmio"`), §2.3.5 `#address-cells`/`#size-cells`, §2.3.6
    `reg` (address+size pairs — where every peripheral's MMIO window
    gets declared).
  - §2.4 Interrupts and Interrupt Mapping — §2.4.1 properties for
    interrupt-generating devices (`interrupts`, `interrupt-parent`),
    §2.4.2 properties for interrupt controllers (`#interrupt-cells`,
    `interrupt-controller`) — this is where the PLIC gets wired to
    every device that raises an interrupt through it.
- **Ch.3 Device Node Requirements** — the required top-level structure:
  - §3.2 Root node.
  - §3.4 `/memory` node — declares installed RAM (base + size).
  - §3.6 `/chosen` Node — where `bootargs` (kernel command line) and
    `stdout-path` (which UART is the console) get set; this is the one
    node most hand-rolled boot flows actually edit per-run.
  - §3.7/§3.8 `/cpus` and `/cpus/cpu*` — per-hart properties, including
    `riscv,isa` (the ISA string this hart implements) and the interrupt
    controller each hart's local interrupt lines connect to.
- **Ch.4 Device Bindings** — the actual driver-specific bindings (UART,
  PLIC, virtio-mmio compatible strings and property sets) live in the
  Linux kernel source tree's `Documentation/devicetree/bindings/`, not
  in the core spec — the core spec defines the *mechanism*, the kernel
  tree defines what each specific device's node must contain.

### What's needed

Don't hand-write this from scratch: QEMU generates a complete, correct
DTB for its exact `virt` machine layout on every boot. Dump the real
one and read it directly —

```
qemu-system-riscv32 -M virt -machine dumpdtb=virt.dtb
dtc -I dtb -O dts virt.dtb -o virt.dts
```

— the same "get the real generated artifact and read it instead of
guessing" approach that found the `sail.json` counteren bug. Since
this project's CLINT already matches QEMU's addresses, that dump is
usable close to verbatim once your own UART/PLIC/virtio-mmio addresses
are chosen to match too.
