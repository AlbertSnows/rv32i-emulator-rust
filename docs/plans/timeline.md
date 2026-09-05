
**Status (2026-09-03): the kernel now boots to the same expected end
state as real hardware** (given no root filesystem, it panics with
`VFS: Unable to mount root fs`, matching real QEMU exactly). Reaching
an actual interactive shell needs a root filesystem,  an initramfs
with a statically-linked busybox,  which surfaced a new requirement
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
left to reach an actual prompt isn't a CPU-correctness bug at all,
it's missing interactive-console I/O plumbing. See
`docs/plans/interactive_console.md`.

**Status (2026-09-04, later): reached a real interactive shell.**
All three `interactive_console.md` items landed (UART RX wiring, the
LSR data-ready bit, the stdin-reader-thread design), which surfaced a
fourth, much deeper bug: `cpu.flags.in_trap`, a single boolean meant
to gate re-entrant interrupt delivery, can't correctly represent
nested traps (an S-mode interrupt handler making a routine SBI ecall
is itself a nested trap),  it gets incorrectly cleared by the *inner*
trap's `mret`/`sret` even though the *outer* handler hasn't finished,
which let a second interrupt sneak in and corrupt the shared
`mepc`/`mcause`/`sepc`/`scause` state, permanently wedging interrupt
delivery (including the previously-reliable timer) the moment it
happened to coincide with the first real PLIC-routed interrupt in the
project's history. Fix: removed the `in_trap` gate entirely from
`select_pending_interrupt`,  the already-correct `sstatus.SIE`/
`mstatus.MIE` checks in `check_interrupt` are the actual, real-hardware
mechanism and don't have this problem. `busybox`'s `ash` now boots to
a `~ #` prompt and correctly runs commands (`ls`, `mkdir`, etc.),
piped or typed live. This was the project's original goal.

Next session isn't new features,  see
`docs/plans/refactor_and_revisit.md` for the cleanup/refactor +
"actually understand what just got fixed" agenda.

2026-09-04, later: with the `C` extension complete, revisited whether
the boot-files build could be simplified (`refactor_and_revisit.md`
§5, now marked done). OpenSBI's ISA string, the kernel's
`CONFIG_RISCV_ISA_C` disable, and musl's `-march` were all narrowed
specifically because the emulator couldn't decode compressed
instructions,  all three rebuilt without that narrowing (OpenSBI/musl
now use `rv32imac_zicsr_zifencei`, the kernel's `C` support is left at
defconfig's default) and reboot to a working interactive shell
confirmed with all three changes together. Also wrote up the
musl+busybox userspace build recipe in `docs/dev/boot_files_setup.md`
for the first time,  it previously existed only in session history.
Two bugs found earlier (`SIP` CSR write, `bus.rs` fault-type
mislabeling,  see `refactor_and_revisit.md` §2) are both fixed.
Remaining refactor/cleanup items (§1, §3, §4 of that doc) are
untouched.