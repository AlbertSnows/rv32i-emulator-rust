# Plans



## Beyond CPU emulation: peripherals / BIOS / OS

(Distant goal, see `project_goal_boot_linux.md` — not near-term
architecture, just a list for later.)

**This entire phase is deliberately modeled on QEMU's `virt` machine** —
not designed from scratch. Peripheral addresses (UART, PLIC, CLINT),
the boot/handoff sequence, and the device tree layout all match what
QEMU's `virt` board already does. The payoff: real, unmodified software
that already targets `virt` (OpenSBI, Linux, generated device trees)
should work here with little to no changes, instead of needing a
bespoke machine description and a custom-ported firmware. See
`docs/dev/peripherals_specs.md`, `docs/dev/peripherals_no_spec.md`, and
`docs/dev/multi_image_loader.md` for the specific addresses/conventions
being matched, each cited back to QEMU's own source
(`hw/riscv/virt.c`, `hw/riscv/boot.c`).

**Peripherals:**

- memory-mapped I/O device model (UART/console minimum)
- external interrupt support (`mip.MEIP`/`SEIP`, PLIC-style or simplified)
- a block/disk device model (if a real root filesystem is ever needed)

**BIOS / firmware:**

Not building one — loading a real, unmodified **OpenSBI** binary, the
same one real hardware/QEMU use. Reimplementing the SBI spec ourselves
would just be re-deriving a solved problem instead of exercising the
emulator; OpenSBI's `generic` platform driver discovers its own
hardware from the device tree it's handed, so matching QEMU's `virt`
addresses/`compatible` strings (see `docs/dev/peripherals_specs.md`,
`docs/dev/peripherals_no_spec.md`) gets a fully spec-correct SBI
implementation with zero firmware-side code of our own. What's
actually needed on this project's side:

- a multi-image loader: place OpenSBI + kernel + DTB in memory at once,
  each at the address it expects (extends the existing ELF loader, not
  a new subsystem)
- correct initial CPU state before jumping to firmware, matching what
  QEMU sets up: `a0` = hart ID, `a1` = DTB physical address, PC =
  firmware entry point
- [done, item #11] full S-mode CSR set (`sepc`, `scause`, `stval`,
  `stvec`, `sstatus`, `sie`, `sip`), SRET, `medeleg`/`mideleg` (trap
  delegation M->S) — this is what OpenSBI itself needs from the CPU to
  run correctly, already in place

**OS (Linux):**

Also not building — loading a real, unmodified Linux kernel image, the
same way. What's needed on this project's side:

- Sv32 virtual memory (`satp`, page-table walker/translation) — needed
  for instruction/load/store page faults (Table 16 codes 12/13/15,
  Chapter 12.3, p.129); Linux relies on virtual memory pervasively
  (per-process address spaces, demand paging, copy-on-write). Also
  confirmed (item #12) to be what's blocking
  `test_rv32ui_p_illegal_passes` and `test_rv32ui_p_dirty_si_passes`
  specifically (`sfence.vma`/`satp`) — not just a distant Linux-boot
  concern, already showing up in near-term conformance testing. See
  `docs/plans/sv32.md` for the satp/PTE layout, the walk algorithm, and
  what needs to change.
- full U/S/M three-mode operation
- timer-driven preemption (`mtime`/`mtimecmp`)
- console + disk drivers matching whatever device model above — these
  are Linux's own existing drivers (8250 for UART, virtio-blk), not
  anything written for this project; they just need real hardware
  underneath that behaves the way those drivers expect

**Status (2026-09-03): the kernel now boots to the same expected end
state as real hardware** (given no root filesystem, it panics with
`VFS: Unable to mount root fs`, matching real QEMU exactly). Reaching
an actual interactive shell needs a root filesystem — an initramfs
with a statically-linked busybox — which surfaced a new requirement
not listed above: real Linux userspace toolchains all assume the "C"
(compressed instructions) extension is available, unlike OpenSBI/the
kernel, which could be told not to use it since this project controls
their builds. See `docs/plans/c_extension.md` for the scope and
implementation plan.

**Status (2026-09-04): the C extension is implemented and busybox now
boots as `/init`.** Fixing it surfaced (and fixed) two real bugs
unrelated to the C extension itself, both never exercised until real
Linux+busybox scheduling activity did: a missing supervisor-timer-
interrupt check in the interrupt-selection logic, and a stale
pre-C-extension `% 4` alignment check plus hardcoded `pc+4` link
address in `j.rs`/`jalr.rs` (both now consolidated into `advance_pc`,
which already had the correct `% 2` check and `advance_amount`). What's
left to reach an actual prompt isn't a CPU-correctness bug at all —
it's missing interactive-console I/O plumbing. See
`docs/plans/interactive_console.md`.

**Status (2026-09-04, later): reached a real interactive shell.**
All three `interactive_console.md` items landed (UART RX wiring, the
LSR data-ready bit, the stdin-reader-thread design), which surfaced a
fourth, much deeper bug: `cpu.flags.in_trap`, a single boolean meant
to gate re-entrant interrupt delivery, can't correctly represent
nested traps (an S-mode interrupt handler making a routine SBI ecall
is itself a nested trap) — it gets incorrectly cleared by the *inner*
trap's `mret`/`sret` even though the *outer* handler hasn't finished,
which let a second interrupt sneak in and corrupt the shared
`mepc`/`mcause`/`sepc`/`scause` state, permanently wedging interrupt
delivery (including the previously-reliable timer) the moment it
happened to coincide with the first real PLIC-routed interrupt in the
project's history. Fix: removed the `in_trap` gate entirely from
`select_pending_interrupt` — the already-correct `sstatus.SIE`/
`mstatus.MIE` checks in `check_interrupt` are the actual, real-hardware
mechanism and don't have this problem. `busybox`'s `ash` now boots to
a `~ #` prompt and correctly runs commands (`ls`, `mkdir`, etc.),
piped or typed live. This was the project's original goal.

Next session isn't new features — see
`docs/plans/refactor_and_revisit.md` for the cleanup/refactor +
"actually understand what just got fixed" agenda.

2026-09-04, later: with the `C` extension complete, revisited whether
the boot-files build could be simplified (`refactor_and_revisit.md`
§5, now marked done). OpenSBI's ISA string, the kernel's
`CONFIG_RISCV_ISA_C` disable, and musl's `-march` were all narrowed
specifically because the emulator couldn't decode compressed
instructions — all three rebuilt without that narrowing (OpenSBI/musl
now use `rv32imac_zicsr_zifencei`, the kernel's `C` support is left at
defconfig's default) and reboot to a working interactive shell
confirmed with all three changes together. Also wrote up the
musl+busybox userspace build recipe in `docs/dev/boot_files_setup.md`
for the first time — it previously existed only in session history.
Two bugs found earlier (`SIP` CSR write, `bus.rs` fault-type
mislabeling — see `refactor_and_revisit.md` §2) are both fixed.
Remaining refactor/cleanup items (§1, §3, §4 of that doc) are
untouched.

## [ ] 14. Refactor CSRState (Postponed)

Attempt after item #8 is done. Motivation: building `mip` surfaced how
much side-effect/special-case behavior `CSRState::write` has
accumulated (address-level read-only vs. field-level read-only within
an otherwise-writable register, internal bypass paths for
cycle/instret/mip that never go through `write()` at all, per-CSR
exceptions like MIP's no-op case). Worth revisiting the whole
read/write/field_for design once interrupts exposes the full shape of
what it actually needs to handle, rather than guessing now.