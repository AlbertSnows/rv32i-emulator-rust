# Exam

A comprehensive self-test. Answer every question, from memory, without
opening the source or the other docs. If you can answer all of these
convincingly, you understand this project well enough to rebuild it
from scratch. This is independent of `review.md`: don't wait for one
to write the other.

No answer key on purpose. Write your answers down somewhere (a scratch
file, out loud, however you want), then bring them back and I'll check
them against the code.

Sections roughly track how deep the questions go, not the codebase
layout. The later sections (testing, peripherals, boot) get more
questions on purpose.

## 1. The pipeline: fetch, decode, execute

1. What does `fetch_word_from_memory` decide, beyond just reading
   bytes? What determines whether it reads 2 or 4 bytes?
2. What is `Format`, and why does every instruction, 16-bit or
   32-bit, end up as one variant of it?
3. Why does `execute()` never need to know whether the instruction it's
   running came from a 2-byte or 4-byte encoding?
4. `core.rs`'s `perform_step` ties fetch, decode, and execute together.
   What else happens in a single step besides those three things?
5. Why is I-type split into six different `Format` variants
   (`LoadType`, `AluImmType`, `JalrType`, `IShiftType`, `SystemType`,
   `CsrType`) even though they all share one 32-bit wire encoding?
   Give a concrete example of two instructions that decode identically
   in shape but must execute completely differently.
6. What is `advance_amount`, where does it come from, and name two
   things that would be wrong if every instruction just always
   advanced the PC by 4.
7. Why do `JType` and `JalrType` need to write a register *and* compute
   a jump target, and why does that logic live in `advance_pc` rather
   than in `execute_j_type`/`execute_i_jalr_type`?

## 2. Instruction formats

8. Draw (in words or ASCII) the bit layout of R-type, I-type, S-type,
   B-type, U-type, and J-type. For each, name what's in the same bit
   position across every other format, and what's unique to it.
9. Why are `rs1`, `rs2`, `rd`, `funct3`, and `opcode` always at the
   same bit position across formats, when the immediate isn't?
10. B-type and J-type both scramble their immediate bits out of
    numerical order instead of storing them contiguously. What's the
    actual reason for that scrambling (not "because the spec says
    so")?
11. Both B-type and J-type immediates represent an offset that's
    always even, and neither one stores bit 0. What's stored there
    instead, and why does the spec choose to do this instead of just
    storing bit 0 directly?
12. **Compute it:** encode `addi x5, x6, -3` as a 32-bit hex value, by
    hand, field by field.
13. **Compute it:** given the hex word `0x00C58463`, identify its
    format and decode every field (opcode, funct3, rd/rs1/rs2 or
    immediate as applicable). What instruction is it?
14. **Compute it:** encode `jal x1, -4` (note: negative offset). Show
    your work for how the sign bit propagates through the scrambled
    immediate fields.

## 3. M and A extensions

15. Why is multiply/divide its own optional extension instead of being
    part of the base ISA, and why does that choice not matter much in
    practice for this project?
16. Why do M-extension instructions not need a new `Format` variant?
17. What does the `A` extension guarantee that plain loads and stores
    don't? Give a concrete scenario where that guarantee matters.
18. Explain the LR/SC (load-reserved/store-conditional) protocol in
    your own words: what does `LR` do, what does `SC` check, and what
    can invalidate a reservation?
19. This project's own docs claim the `A` extension was *easier* to
    implement here than the spec's framing suggests. Why?

## 4. The C extension

20. What does "decode-time expansion" mean for the `C` extension, and
    why did it require zero new execution code?
21. Name the register-field encoding difference between `CR`/`CI`/
    `CSS` formats and `CIW`/`CL`/`CS`/`CA`/`CB` formats. Why do the
    latter only reach 8 registers instead of 32?
22. What is a HINT, in RVC terms, and how is it different from a
    reserved/illegal encoding? Give the specific example from this
    project where the two were confused, and explain why the bug's
    symptom (a trap-mode mismatch) pointed at the wrong layer of the
    system.
23. Why does variable-width fetch matter for alignment? Specifically:
    why can a 32-bit instruction legally start at a 2-byte-aligned
    address once `C` is enabled, and what would break if the fetch
    logic assumed 4-byte alignment always?
24. **Compute it:** `C.ADDI` and `C.LI` share the same rd/imm field
    layout in different ways. Pick one and, given a specific compressed
    hex halfword of your choosing, decode it field-by-field.

## 5. Privileged architecture and modes

25. Name the three privilege levels this project implements, their
    relative ranking, and why one particular numeric level is skipped
    entirely.
26. What determines the minimum privilege level required to access a
    given CSR, and where does that information live in the CSR
    address itself?
27. What is `medeleg`/`mideleg` for? Why do interrupts check one and
    exceptions check the other?
28. Walk through, step by step, what happens in hardware terms when an
    S-mode program executes `ecall`: what gets saved, what mode you
    end up in, and how you get back.
29. What do `MPP`/`SPP` do, and why does `MRET`/`SRET` need to consult
    them specifically (as opposed to just always returning to
    M-mode/S-mode)?
30. Why can an M-mode interrupt safely preempt S-mode code, including
    an S-mode trap handler already running, with no extra bookkeeping,
    while a same-privilege-level nested trap is dangerous without care?

## 6. CSR mechanics

31. What's the difference between a CSR address being entirely
    read-only versus a CSR being writable overall but having specific
    read-only *bits* within it? Give an example of the second kind
    from this project.
32. Why does `cycle`/`instret` bypass the normal `write()` path
    entirely, and why does `MTVEC` force specific low bits regardless
    of what's written to it?
33. For an instruction like `csrrs`, where does the "old value" it
    returns come from, and why doesn't the return value of
    `guest_write` itself matter for that?
34. Explain the exact shape of the bug that hit both `MIP` and `SIP`'s
    write handling. What did the code compute correctly, and what did
    it then fail to do with that value?
35. **Compute it:** given `mie` has only the machine-timer-interrupt
    bit set, `mip` has both the machine-timer and machine-external
    bits set, and the CPU is in M-mode with `mstatus.MIE=1`, which
    interrupt (if any) gets taken?

## 7. Interrupts, and the hardest bug in the project

36. What are `mip` and `mie` for, respectively, and how does an
    interrupt's `mcause` value differ from an exception's?
37. Describe `select_pending_interrupt`'s priority order, and explain
    why priority even needs to exist (i.e., what could happen if two
    interrupt sources were pending at once with no defined order).
38. State `check_interrupt`'s privilege-level rule in your own words:
    when is an interrupt always taken regardless of the enable bit, and
    when does the enable bit matter?
39. Without looking anything up: explain the full mechanism of the
    `in_trap` bug. What was `in_trap` trying to protect against, why
    was a single boolean insufficient, what specific sequence of
    events corrupted CPU state, and why did it take so long to
    diagnose?
40. What protects against the failure `in_trap` was trying (and
    failing) to prevent, on hardware and in this emulator now? Why is
    that mechanism sufficient by itself?
41. Why was removing `in_trap`'s gate the right fix, instead of
    replacing it with a nesting counter?
42. What misled the initial investigation into this bug, and what
    piece of evidence (not guesswork) is what pointed at the true
    mechanism?

## 8. Sv32 virtual memory

43. What problem does virtual memory solve that this project didn't
    need to care about before Sv32 was implemented?
44. Walk through the two-level Sv32 page table walk from `satp` to a
    final physical address, step by step.
45. Name every PTE permission/status bit this project implements and
    what each one means.
46. What makes a PTE's R/W/X combination reserved rather than valid,
    and what should happen if the CPU encounters one?
47. When does address translation get bypassed entirely, even with
    `satp.MODE` set to enable it?
48. Why does a page fault need its own three `TrapCause` variants
    instead of reusing the existing load/store/instruction-fault
    causes?

## 9. Peripherals: PLIC

49. What problem is the PLIC solving that `mip`/`mie` alone can't?
50. Walk through the full journey of one external interrupt from
    "device becomes pending" to "handler runs", naming every PLIC
    field involved (`priority`, `pending`, `enabled`, `threshold`,
    `armed`).
51. What does `armed` represent, and why is it a separate piece of
    state from `pending` rather than the same thing?
52. Explain the reasoning behind `compute_eip`'s condition in your own
    words, not the code: what has to be true for a given context for
    an external interrupt to be visible at all?
53. What's the difference between `claim` and `complete`, and why does
    an OS's interrupt driver need both instead of just one
    acknowledgment step?
54. **Compute it:** the PLIC's enable-bits region starts at offset
    `0x2000` with a stride of `0x80` per context. What MMIO offset
    would you read to get the raw 32-bit enable word covering sources
    0-31 for context 1 (S-mode)?

## 10. Peripherals: UART and SBI

55. Name every 16550 UART register this project implements, and what
    each one is for.
56. What is the LSR data-ready bit, and describe the exact bug that
    existed here for a while: what did the register report versus
    what it should have reported, and why did that produce a silent
    hang instead of a crash?
57. Trace the full path a keystroke takes from the host's terminal to
    a value the guest kernel's UART driver can read. Name every piece
    involved (there's a thread in this path; why does it need to be
    one?).
58. What is OpenSBI, and why does this project load an unmodified
    binary instead of implementing the SBI spec itself?
59. What has to be true about this emulator's memory map for OpenSBI's
    `generic` platform driver to work with zero custom platform code?

## 11. The loader and boot sequence

60. Name the three files `boot_kernel` loads, and what each one
    contributes to getting from power-on to a running kernel.
61. What CPU/register state has to be correct *before* jumping to
    OpenSBI's entry point for the handoff to work at all?
62. Why does the device tree blob need two specific edits away from
    what QEMU generates, and what would go wrong with each one left
    unedited?
63. What's `earlycon` for, and why was it essential for debugging
    early boot failures specifically (as opposed to failures deeper
    into Linux's own boot)?
64. Why does this project's peripheral and boot design deliberately
    copy QEMU's `virt` machine instead of inventing its own machine
    layout? What two concrete capabilities does that decision buy,
    beyond convenience?

## 12. Building the boot files (OpenSBI, kernel, musl, busybox)

65. Why can't OpenSBI be built with the same bare-metal GCC toolchain
    used elsewhere in this project?
66. Why does the kernel need a specific older tag instead of the tip
    of `master`?
67. Of the kernel config options that get disabled, name at least
    three, and for each say whether it was disabled *because this
    emulator can't decode/handle it* versus *because the hardware it
    models doesn't exist here* versus *because of an unrelated
    toolchain bug*. Those are different reasons; don't collapse them.
68. Why did `F` (floating point) declared in the riscv-arch-test UDB
    config once break booting even for test suites that have nothing
    to do with floating point?
69. What problem does the `musl-gcc.specs.sh`-generated specs file
    solve, and what goes wrong if you skip it?
70. Why does `CC` have to be passed on busybox's `make` command line
    instead of being exported as an environment variable?
71. Why does the initramfs need a hand-written device-node file
    (`nod` entries) at all, instead of just letting the kernel's
    `devtmpfs` create `/dev/console` on its own?
72. Once the `C` extension existed, which specific ISA-narrowing
    decisions became unnecessary, and which ones stayed exactly as
    narrow as before regardless? For each one that stayed narrow,
    explain why `C` support didn't change the reasoning.

## 13. Testing infrastructure

73. Name this project's three layers of correctness testing, and for
    each one, describe the *kind* of bug it's specifically good at
    catching that the others aren't.
74. What is the `tohost`/`fromhost` protocol, and how does a test
    binary communicate pass/fail using it?
75. Walk through what `arch_test_runner`'s main loop does, start to
    finish, including the character-output special case.
76. Why does `build.rs` generating one `#[test]` per fixture file work
    better here than one hand-written test function per instruction?
77. What is riscv-arch-test/ACT4, and how is its actual DUT contract
    different from what an older tool like RISCOF expected?
78. What is a UDB config declaring, concretely, and why did adding `C`
    to this project's UDB config fail the first time, and what two
    things did it need before it validated?
79. Explain, in your own words, why 96 of 97 generated `Zca` tests
    passing and one failing was a meaningful, trustworthy result rather
    than a fluke.
80. This project explicitly avoids trusting a "N/N passing" claim on
    faith. What mechanism exists so that claim can always be
    independently re-verified, and what does it re-run?

## 14. Putting it together

81. If someone deleted this entire repository and handed you only this
    exam and your own memory, what is the *first* subsystem you'd
    implement, and why does the order matter?
82. Name the single hardest bug in this project's history and explain
    it end to end, unprompted, in under two minutes.
83. What's still explicitly unfinished or acknowledged as a wart in
    this codebase right now, and why was it left that way rather than
    fixed immediately?
