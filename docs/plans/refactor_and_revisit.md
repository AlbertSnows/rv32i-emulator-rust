# Refactor + revisit session

The emulator now boots real, unmodified Linux + busybox to a working
interactive shell — the project's original goal. This is the agenda
for the next session, which isn't about new features: it's cleanup
(accumulated cruft, some of it from very deep debugging today) plus
actually understanding, not just having working, several pieces that
got fixed under time pressure without a real walkthrough.

## 1. Cruft to clean up

Concrete, mechanical items — no design decisions needed, just doing them:

- **`src/bin/debug_boot.rs`** — the throwaway instrumented-boot tool
  built today for the interrupt/UART investigation (symbol resolution,
  trap-cause tallying, PLIC state dumps, the stdin-threading
  duplicate). Decide: delete it, or keep it as a real diagnostic tool
  (if kept, it needs its own cleanup — right now it's a pile of
  one-off instrumentation bolted on in sequence, not something
  designed to be reused).
- **`src/loader.rs:61`** — `TEXT_OFFSET_LOCATION` const and its read
  are commented-out dead code (the kernel-placement formula stopped
  using it once the real QEMU-matching `align_up` formula was found).
  Delete outright rather than leave commented.
- **`src/cpu/instructions/j.rs` / `src/cpu/instructions/i/jalr.rs`** —
  `execute_j_type`/`execute_i_jalr_type` are now empty
  (`Ok(ExecutionSignal::Continue)`) with the real logic commented out
  above the `Ok(...)` line, since that logic moved into `advance_pc`.
  Delete the commented code and decide whether these functions (and
  their dispatch from `Format::execute`) should be removed entirely
  rather than kept as no-ops.
- **`cpu.flags.in_trap`** (`src/cpu/definitions/cpu/flags.rs`,
  `CPUFlags`) — the one thing that *read* this flag
  (`select_pending_interrupt`'s gate) is now commented out, since it
  was the actual bug (see §3 below). It's still *written* by
  `handle_trap`/`inst_i_xret`, and three tests
  (`test_handle_trap_sets_in_trap_flag`,
  `test_inst_i_mret_clears_in_trap_flag`,
  `test_inst_i_sret_clears_in_trap_flag`) still assert on it — but
  nothing reads it anymore. Decide whether to remove it entirely
  (`CPUFlags` would then have nothing left in it) or keep it as inert
  bookkeeping for some future use.

## 2. Real bugs found today — fixed

Found while working on the timer/UART chain, deferred briefly since
they weren't blocking, then fixed on 2026-09-04:

- **`src/cpu/definitions/cpu/csr.rs`** — `guest_write`'s `SIP` arm had
  the *exact* bug the `MIP` arm had before today's earlier fix: it
  computed a masked return value but never assigned to `*property`, so
  any S-mode write to its own `sip` (e.g. clearing `SSIP` for a
  software interrupt) was silently discarded. Fixed the same shape as
  `MIP`'s arm (mask `value` to the writable bits via the new
  `masks::PER_SOURCE_SIP`, merge with the preserved bits, assign back).
- **`src/cpu/definitions/cpu/bus.rs`** — `direct_write`'s out-of-bounds
  catch-all returned `TrapCause::LoadAccessFault` on the *write* path;
  now correctly returns `StoreAccessFault`. The one test asserting the
  old (wrong) fault type was updated, not deleted — its actual intent
  (an out-of-range write falls through cleanly rather than being
  swallowed as a UART access) still holds.

## 3. `CSRState` refactor (pre-existing item, now more urgent)

Already tracked in `docs/plans/plans.md` item 14, written before any
of today's work:

> Motivation: building `mip` surfaced how much side-effect/special-case
> behavior `CSRState::write` has accumulated (address-level read-only
> vs. field-level read-only within an otherwise-writable register,
> internal bypass paths for cycle/instret/mip that never go through
> `write()` at all, per-CSR exceptions like MIP's no-op case).

That "MIP's no-op case" is *precisely* the discarded-write bug fixed
today (§4 below) — direct, concrete evidence the special-casing had
already gotten out of hand before we even knew about the interrupt
bugs it was hiding. Worth actually doing this refactor now, with a
much clearer picture of what `guest_write` actually needs to support
(uniform bit-masked writes for the SSIP/STIP/SEIP-style CSRs, hardwired
read-only fields computed elsewhere, the CYCLE/INSTRET bypass, MTVEC's
mode-forcing) than existed when this item was first written.

## 4. Understanding session: what actually happened today

The user's own words: "there's also a lot of unknowns remaining.
specifically, me not understanding how/why we did specific things."
This is the list of what's worth walking through properly rather than
just having working:

- **The `in_trap` bug itself.** Why a single boolean can't represent
  trap nesting; why real hardware doesn't need an equivalent concept
  at all (`sstatus.SIE`/`mstatus.MIE` already do the whole job); why
  cross-privilege-level nesting (an M-mode interrupt firing while
  S-mode code, including an S-mode handler, runs) is safe without any
  extra bookkeeping (separate `mepc`/`mcause` vs `sepc`/`scause`) while
  same-level nesting isn't. This is genuinely one of the harder
  pieces of RISC-V privileged-mode reasoning in the whole project —
  worth a real walkthrough with a concrete timeline diagram of the
  nested-trap corruption, not just "we removed a line and it worked."
- **The C extension's two real subtleties** — register remapping
  (`x8`-`x15` only for CIW/CL/CS/CA/CB formats vs. full 5-bit for
  CR/CI/CSS) and the scrambled-immediate reassembly per instruction
  (`docs/plans/c_extension.md` already documents *what* the scrambling
  is per instruction, but a revisit session should cover *reading* an
  RVC encoding figure from the spec directly and deriving the
  mask/shift sequence by hand, not just trusting the already-written
  code).
- **PLIC's claim/complete/armed protocol**
  (`src/peripherals/plic.rs`) — why `armed` exists as a separate latch
  from `pending`, what it's modeling (a level-triggered interrupt
  that shouldn't re-fire until the handler acknowledges it), and how
  `claim()`/`complete()` map to the real SiFive PLIC's MMIO protocol
  the kernel's generic IRQ driver expects.
- **`handle_trap`'s delegation logic** (`src/cpu/core.rs`) — the
  `register_value`/`corresponding_mask` matches, why interrupts check
  `mideleg` and exceptions check `medeleg`, why the mask for a given
  cause has to match its *delegation-register* bit position (which
  happens to equal the cause code's own numbering by spec design) —
  and why adding `SupervisorTimerInterrupt` required updating three
  separate places (`select_pending_interrupt`'s array,
  `handle_trap`'s two matches) rather than being a one-line change.
- **`advance_pc` owning both the jump target *and* the link-register
  write** for `JType`/`JalrType` — why this consolidation was the
  right call (both need `advance_amount`, which `execute_j_type`/
  `execute_i_jalr_type` never had access to) versus the alternative
  (threading instruction width through `Format::execute`).
- **Why `guest_write`'s payload return value doesn't matter** (only
  `Ok`/`Err` does — the real "old value" for `csrrs`/`csrrc`/etc.
  comes from a separate `read()` call before the write) — small, but
  came up confusingly during the MIP fix and is worth being clear on
  before touching `csr.rs` again for the `SIP` fix or the bigger
  refactor.

## 5. Now that C exists: does the build get simpler? — done, 2026-09-04

Resolved. `docs/dev/boot_files_setup.md` is updated:

- OpenSBI's `PLATFORM_RISCV_ISA` narrowed to `rv32imac_zicsr_zifencei`
  (added `c`) — rebuilt and confirmed working.
- The kernel's `CONFIG_RISCV_ISA_C` disable is removed entirely (left
  at defconfig's default, enabled) — rebuilt and confirmed working,
  with the `EFI`-selects-`RISCV_ISA_C` rationale updated to explain
  that interaction is now moot (`EFI` stays disabled on its own
  merits, unrelated to `C`).
- The DTB's `riscv,isa`/`riscv,isa-extensions` narrowing is
  *unchanged* — confirmed by testing that it still works without `c`
  added, since (unlike `f`/`d`/`sstc`) nothing actually reads that
  property for `C` support; the kernel's `C` usage is gated by
  `CONFIG_RISCV_ISA_C` at compile time, not the DTB. Documented as a
  deliberate choice, not an oversight.
- musl rebuilt with unified `-march=rv32imac_zicsr_zifencei` (previously
  `rv32ima_zicsr_zifencei`, mismatched from busybox's own flags) —
  confirmed the mismatch was never load-bearing (musl's own compiled
  objects didn't need `C` avoidance either — prebuilt userspace
  toolchains, including this one, generally emit compressed
  instructions unconditionally regardless of `-march`, which is *why*
  this was the one piece C-avoidance couldn't actually reach). Now
  consistent for documentation clarity, not because it fixed a bug.
- The musl+busybox build recipe — previously undocumented, existing
  only in session history — is now written up as a new §3 in
  `docs/dev/boot_files_setup.md` (musl build, the `musl-gcc.specs.sh`
  wrapper, kernel UAPI headers, busybox build/install, the
  `initramfs-devnodes.txt` device-node file, `CONFIG_INITRAMFS_SOURCE`
  wiring).

All three rebuilt pieces (OpenSBI, kernel, busybox/musl) were verified
together in one real boot to an interactive, repeatedly-responsive
shell before the docs were written — not just individually compiled.

One incidental finding, left unfixed (out of scope for this item):
`docs/dev/boot_files_setup.md`'s DTB step says the output is
`~/opt/virt.dtb`, and that's what `loader.rs`'s `dtb_location` should
point at — but the actual `loader.rs` in the repo points at
`~/opt/virt_earlycon_narrowed.dtb`, a differently-named file from an
earlier iteration of the recipe. Both exist on disk and the real one
works; the doc's filename just doesn't match. Worth a quick fix
(rename the file to match the doc, or update the doc/loader.rs to
match reality) but unrelated to the C extension.
