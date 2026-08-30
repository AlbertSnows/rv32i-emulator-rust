# Plans



## Beyond CPU emulation: peripherals / BIOS / OS

(Distant goal, see `project_goal_boot_linux.md` — not near-term
architecture, just a list for later.)

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

## [ ] 14. Refactor CSRState (Postponed)

Attempt after item #8 is done. Motivation: building `mip` surfaced how
much side-effect/special-case behavior `CSRState::write` has
accumulated (address-level read-only vs. field-level read-only within
an otherwise-writable register, internal bypass paths for
cycle/instret/mip that never go through `write()` at all, per-CSR
exceptions like MIP's no-op case). Worth revisiting the whole
read/write/field_for design once interrupts exposes the full shape of
what it actually needs to handle, rather than guessing now.