# Exam

A comprehensive self-test. This also acts as a useful Q&A for certain
design decisions. 

Sections roughly track how deep the questions go, not the codebase
layout. The later sections (testing, peripherals, boot) get more
questions on purpose.

## 1. The pipeline: fetch, decode, execute

1. What does `fetch_word_from_memory` decide, beyond just reading
   bytes? What determines whether it reads 2 or 4 bytes?
   2. it decides if the instruction is a half word. it looks at the first few bits.
2. What is `Format`, and why does every instruction, 16-bit or
   32-bit, end up as one variant of it?
   3. Format is the different structures a 32bit instruction comes in. Our CPU is a 32bit
processor, so it expects all instructions to fit that structure. The Format type outlines
what the accepted instruction shapes are. 
3. Why does `execute()` never need to know whether the instruction it's
   running came from a 2-byte or 4-byte encoding?
   4. because we decode 16bit instructions into 32 bit ones. 
4. `core.rs`'s `perform_step` ties fetch, decode, and execute together.
   What else happens in a single step besides those three things?
   5. in terms of just executing an instruction, advancing pc
5. Why is I-type split into six different `Format` variants
   (`LoadType`, `AluImmType`, `JalrType`, `IShiftType`, `SystemType`,
   `CsrType`) even though they all share one 32-bit wire encoding?
   Give a concrete example of two instructions that decode identically
   in shape but must execute completely differently.
   6. Because I type is the one that breaks the rule. jalr has op code 1100111, addi
   has the op code 0010011. Format is defined by the op code, so I type decomposes into
   the different instruction shapes that are defined per op code. Regarding two different
   instructions, I will again cite jalr and addi.
6. What is `advance_amount`, where does it come from, and name two
   things that would be wrong if every instruction just always
   advanced the PC by 4.
   7. advance amount decides if pc moves forward by 2 or 4. many instructions are encoded
   in a sequence of 16bits, so we can double the number of instructions in a given related
   range. For such instructions, we want to advance by 2 not 4. If we moved forward by 4,
   we risk skipping a necessary instruction. It also misaligns future instructions.
7. Why do `JType` and `JalrType` need to write a register *and* compute
   a jump target, and why does that logic live in `advance_pc` rather
   than in `execute_j_type`/`execute_i_jalr_type`?
   8. they store the pc info, and pc is kept separate from our execution space, so
   it is deemed more appropriate to move the logic into pc advancement. 
        if you wrote the register in execution, you risk clobbering rs1 before
        the jump target was correctly calculated. 

## 2. Instruction formats

8. Draw (in words or ASCII) the bit layout of R-type, I-type, S-type,
   B-type, U-type, and J-type. For each, name what's in the same bit
   position across every other format, and what's unique to it.
   9. I'm not gonna do that. the structure of each is outlined in rvalp.pdf
9. Why are `rs1`, `rs2`, `rd`, `funct3`, and `opcode` always at the
   same bit position across formats, when the immediate isn't?
   10. Because it's more convenient to keep these at a specified location. Imm
    is the "remainder" leftover consequence. It is meant to suit different purposes
    depending on the instruction context. As such, it has no appropriate set bit
    position. Instead, its definition is kept consistent within a given format. 
10. B-type and J-type both scramble their immediate bits out of
    numerical order instead of storing them contiguously. What's the
    actual reason for that scrambling (not "because the spec says
    so")?
    11. It's so the sign bit always lands at bit 32. 
11. Both B-type and J-type immediates represent an offset that's
    always even, and neither one stores bit 0. What's stored there
    instead, and why does the spec choose to do this instead of just
    storing bit 0 directly?
    12. It just stores a 0. 
12. **Compute it:** encode `addi x5, x6, -3` as a 32-bit hex value, by
    hand, field by field.
    13. structure is `mnemonic rd, rs1, imm`
    13. imm[11:0] | rs1 | funct3 | rd | op code | 
    14. rd = x5 = 0b101, rs1 = 0b110, -3 = 1111_1111_1111_1111_1111_1111_1111_1101
    15. op code = 001_0011
    16. funct3 = 000
    17. |111111111101|_0011_0|000|_0010_1|[001_0011]
13. **Compute it:** given the hex word `0x00C58463`, identify its
    format and decode every field (opcode, funct3, rd/rs1/rs2 or
    immediate as applicable). What instruction is it?
    14. 0000_0000 | C5 | 84 | 63 = 0000_0000 | 1100_0101 | 1000_0100 | 0110_0011
    15. [0000_000][0_1100]_[0101_1][000]_[0100_0][110_0011]
    16. it's b type, beq
14. **Compute it:** encode `jal x1, -4` (note: negative offset). Show
    your work for how the sign bit propagates through the scrambled
    immediate fields.
    15. [imm[20|10:1|11|19:12]][_0000_1][110_1111]
    16. -4 = 0b100, 1111_1111_1111_1111_1100
    17. [1|111_1111_100|1|_1111_1111]
    18. final output = [1111_1111_1001_1111_1111][_0000_1][110_1111]

## 3. M and A extensions

15. Why is multiply/divide its own optional extension instead of being
    part of the base ISA, and why does that choice not matter much in
    practice for this project?
    16. technically multiply and divide can be implemented by add/subtract
    17. some chips do not need/want to use mult/div. for us it's not expensive to add.
16. Why do M-extension instructions not need a new `Format` variant?
    17. because they already conform to R Type
17. What guarantee does the A extension provide that ordinary loads and stores lack? 
    Give a concrete scenario where that guarantee matters.
    18. It's atomic. So a full operation will complete, where other 
        instructions may not. For example, a read, modify, write instruction sequence
        might fail in such a way that its value is stale by the time it reaches load
18. Explain the LR/SC (load-reserved/store-conditional) protocol in
    your own words: what does `LR` do, what does `SC` check, and what
    can invalidate a reservation?
    19. LR reads a place in memory and keeps track of that address. 
    20. SC only writes if LR is unchanged when it runs. 
    21. sc.w invalidates a reservation
19. This project's own docs claim the `A` extension was *easier* to
    implement here than the spec's framing suggests. Why?
    20. we only have one hart

## 4. The C extension

20. What does "decode-time expansion" mean for the `C` extension, and
    why did it require zero new execution code?
    21. I don't understand what this question is asking completely, but C
        just decodes into an existing format, so we just need to write parsers.
21. Name the register-field encoding difference between `CR`/`CI`/
    `CSS` formats and `CIW`/`CL`/`CS`/`CA`/`CB` formats. Why do the
    latter only reach 8 registers instead of 32?
    22. Due to size constraints. The latter need more room for the other components.
22. What is a HINT, in RVC terms, and how is it different from a
    reserved/illegal encoding? Give the specific example from this
    project where the two were confused, and explain why the bug's
    symptom (a trap-mode mismatch) pointed at the wrong layer of the
    system.
    23. hint is used for no-op behaviors to be picked up without interfering with
        the cpu system. 
    24. c_lui faulted because imm==0 didn't exclude rd==0, which is a valid hint case
23. Why does variable-width fetch matter for alignment? Specifically:
    why can a 32-bit instruction legally start at a 2-byte-aligned
    address once `C` is enabled, and what would break if the fetch
    logic assumed 4-byte alignment always?
    24. it would skip various C instructions and treat valid code as misaligned
24. **Compute it:** `C.ADDI` and `C.LI` share the same rd/imm field
    layout in different ways. Pick one and, given a specific compressed
    hex halfword of your choosing, decode it field-by-field.
    25. 0x05_15 = 0000_0101_0001_0101
    26. funct3 = 000
    27. imm signed = 0
    28. rd = 0x1010
    29. imm = 00101
    30. quadrant = 01
    31. group = (01, 000) = C.ADDI/NOP
    32. imm_high = 0
    33. imm_low = 00101 = 5
    34. c.addi x10 x10 5 = addi x10 x10 5

## 5. Privileged architecture and modes

25. Name the three privilege levels this project implements, their
    relative ranking, and why one particular numeric level is skipped
    entirely.
    26. M > S > U, where > indicates the left side is more important, we don't
        have a use case for U type. 
26. What determines the minimum privilege level required to access a
    given CSR, and where does that information live in the CSR
    address itself?
    27. two bits, at the 8th position
27. What is `medeleg`/`mideleg` for? Why do interrupts check one and
    exceptions check the other?
    28. they control checks and exceptions, used to discern the appropriate mode.
        one bit represents a trap cause. lets m mode delegate handling specific causes
        down to other modes.
28. Walk through, step by step, what happens in hardware terms when an
    S-mode program executes `ecall`: what gets saved, what mode you
    end up in, and how you get back.
    29. ecall has to do with calls to various modes. presumably s mode gets saved, and
        the next call gets made in m mode.
    30. mepc gets the current pc
    31. mcause records call from mode
    32. pp records where trap came from
    33. pc jumps to tvec
    34. get back via ret
29. What do `MPP`/`SPP` do, and why does `MRET`/`SRET` need to consult
    them specifically (as opposed to just always returning to
    M-mode/S-mode)?
    30. they hold the previous privilege. we need to be able to return to whatever
        the previous privilege was
30. Why can an M-mode interrupt safely preempt S-mode code, including
    an S-mode trap handler already running, with no extra bookkeeping,
    while a same-privilege-level nested trap is dangerous without care?
    31. because m mode is higher priority. we can't lose anything by switching to it
        but the same is not true for s mode. we may risk losing the mode we were in.
    32. they have separate trap registers. (mstatus vs sstatus)

## 6. CSR mechanics

31. What's the difference between a CSR address being entirely
    read-only versus a CSR being writable overall but having specific
    read-only *bits* within it? Give an example of the second kind
    from this project.
    33. MIP only has certain bits accessible; msip
32. Why does `cycle`/`instret` bypass the normal `write()` path
    entirely, and why does `MTVEC` force specific low bits regardless
    of what's written to it?
    33. because they just exist to keep track of the number of cycles. 
        they are unwritable. 
    34. we do not support vectored mode, so we force the first two bits to be 00
33. For an instruction like `csrrs`, where does the "old value" it
    returns come from, and why doesn't the return value of
    `guest_write` itself matter for that?
    34. it comes from the csr. we don't use the value from guest write
34. Explain the exact shape of the bug that hit both `MIP` and `SIP`'s
    write handling. What did the code compute correctly, and what did
    it then fail to do with that value?
    35. it didn't write it to property. 
35. **Compute it:** given `mie` has only the machine-timer-interrupt
    bit set, `mip` has both the machine-timer and machine-external
    bits set, and the CPU is in M-mode with `mstatus.MIE=1`, which
    interrupt (if any) gets taken?
    36. mti = 7. bit 7 is set. mti gets taken. 

## 7. Interrupts, and the hardest bug in the project

36. What are `mip` and `mie` for, respectively, and how does an
    interrupt's `mcause` value differ from an exception's?
    37. mip is used to keep track of what's pending
    38. mie is used to keep track of what we allow to interrupt
    39. mcause interrupt has top bit set to 1, exception's is set to 0
37. Describe `select_pending_interrupt`'s priority order, and explain
    why priority even needs to exist (i.e., what could happen if two
    interrupt sources were pending at once with no defined order).
    38. mei > mti > sei > sti
    39. we want to handle important (M) ones first. if there was no order, we'd have
        no deterministic way of knowing how interrupts are handled. 
38. State `check_interrupt`'s privilege-level rule in your own words:
    when is an interrupt always taken regardless of the enable bit, and
    when does the enable bit matter?
    39. if current level is lower than target, or if they're the same, but the
        global field is toggled, it must be handled
39. Without looking anything up: explain the full mechanism of the
    `in_trap` bug. What was `in_trap` trying to protect against, why
    was a single boolean insufficient, what specific sequence of
    events corrupted CPU state, and why did it take so long to
    diagnose?
    40. in trap was trying to protect against double trap. however, with more
        development, it became possible to have nested traps that didn't break the
        system, so it became irrelevant. 
    41. insufficient because an s mode interrupt making an sbi ecall is a nested
        trap, but is not incorrect. 
    42. a second interrupt clobbered the previous state info
    43. the debug_boot had the wrong sstatus address assumption
    44. identified because trap cause tally showed SEI and STI froze
        the exact same moment. this shows that this wasn't one starved source.
        instead, the delivery itself had stopped. 
40. What protects against the failure `in_trap` was trying (and
    failing) to prevent, on hardware and in this emulator now? Why is
    that mechanism sufficient by itself?
    41. IE, via set pie. entering any trap already disables further same-or-lower 
        interrupts automatically. 
41. Why was removing `in_trap`'s gate the right fix, instead of
    replacing it with a nesting counter?
    42. because we already had a system in place to handle nested trapping
42. What misled the initial investigation into this bug, and what
    piece of evidence (not guesswork) is what pointed at the true
    mechanism?
    43. the claim was SEI can never fire, but was a red herring. 
        the trap cause showed two independent interrupt sources freezing at
        the same time. this is a systemic failure, not per-source.

## 8. Sv32 virtual memory

43. What problem does virtual memory solve that this project didn't
    need to care about before Sv32 was implemented?
    44. it solves giving a program access to a virtual memory space so different programs 
        don't overwrite each other.
44. Walk through the two-level Sv32 page table walk from `satp` to a
    final physical address, step by step.
    45. we get the root table from memory, we get the leaf table from root. 
    46. we expect various fields (e.g. pte.valid) to be configured correctly
    47. we expect the sum status bit to be set
    48. if root_pte is a leaf, it's a super page. we need to check more
        config options, update A/D bits, check perms, then pull the final
        address from ppn one. 
    49. if root_pte is not the leaf, then we repeat above, but for the leaf
        index. 
45. Name every PTE permission/status bit this project implements and
    what each one means.
    46. valid, readable, writable, executable, user_accessible, accessed, dirty
    47. they speak for themselves
46. What makes a PTE's R/W/X combination reserved rather than valid,
    and what should happen if the CPU encounters one?
    47. whether or not it's a valid state. if not, page fault.
47. When does address translation get bypassed entirely, even with
    `satp.MODE` set to enable it?
    48. M mode
48. Why does a page fault need its own three `TrapCause` variants
    instead of reusing the existing load/store/instruction-fault
    causes?
    49. those are access faults, which is a different problem. 
    50. access faults are significant, but page faults may be recoverable

## 9. Peripherals: PLIC

49. What problem is the PLIC solving that `mip`/`mie` alone can't?
    50. many problems. it accounts for priority, pending, enabled, threshold, and armed
    51. plic tells you which device fired, handles prioritization of interrupts,
        and handles enable/disable of devices. 
50. Walk through the full journey of one external interrupt from
    "device becomes pending" to "handler runs", naming every PLIC
    field involved (`priority`, `pending`, `enabled`, `threshold`,
    `armed`).
    51. a byte is received, it is set to pending, armed is set to false
    52. compute eip gates checking for pending further
        53. check that priority beats the threshold
    52. an interrupt is read, and claimed
    53. it is no longer pending
    54. it is marked complete, and rearmed
51. What does `armed` represent, and why is it a separate piece of
    state from `pending` rather than the same thing?
    52. armed represents whether a source is allowed to raise a new
        pending request. 
52. Explain the reasoning behind `compute_eip`'s condition in your own
    words, not the code: what has to be true for a given context for
    an external interrupt to be visible at all?
    53. the condition asks if there's any source id that has a pending bit
        is enabled, and is important enough to be cared about. it needs
        a source that's pending, enabled, and above threshold. 
53. What's the difference between `claim` and `complete`, and why does
    an OS's interrupt driver need both instead of just one
    acknowledgment step?
    54. claim picks up an interrupt and clears current pending flag. 
    55. complete allows that pending source location to be processed again
        and resets armed. 
54. **Compute it:** the PLIC's enable-bits region starts at offset
    `0x2000` with a stride of `0x80` per context. What MMIO offset
    would you read to get the raw 32-bit enable word covering sources
    0-31 for context 1 (S-mode)?
    55. 0x2000 = enable base
    56. 0x80 = stride
    57. context = 1
    58. = 2081
    59. word index = 0
    60. offset = 2080

## 10. Peripherals: UART and SBI

55. Name every 16550 UART register this project implements, and what
    each one is for.
    56. offset 0 gives you the next byte, or writes it to terminal.
    57. offset 5 is lsr, communicates if there's a byte pending
56. What is the LSR data-ready bit, and describe the exact bug that
    existed here for a while: what did the register report versus
    what it should have reported, and why did that produce a silent
    hang instead of a crash?
    57. LSR = line status register. keeps track of a byte sitting on the register.
    58. the uart reported a hardcoded value that indicated nothing was waiting from
        the buffer, so there was nothing to crash.
57. Trace the full path a keystroke takes from the host's terminal to
    a value the guest kernel's UART driver can read. Name every piece
    involved (there's a thread in this path; why does it need to be
    one?).
    58. stdin is blocking. we put it in a thread. we send it to tx. 
        rx gets it. we put it in the uart and plic state machine. 
        eventually it gets handled, we remove the character from uart, and we
        reset the plic state for that source id. 
58. What is OpenSBI, and why does this project load an unmodified
    binary instead of implementing the SBI spec itself?
    59. sbi is the mediator/supervisor between the cpu and OS. We didn't need to
        implement an sbi because OpenSBI works for us. it has a large complex
        spec. 
59. What has to be true about this emulator's memory map for OpenSBI's
    `generic` platform driver to work with zero custom platform code?
    60. it has to emulate a riscv cpu in the way open sbi expects it to, and also add 
        instructions to communicate needed info for sbi to perform handoffs. 
    61. needs to match a real, already supported platform memory map. 

## 11. The loader and boot sequence

60. Name the three files `boot_kernel` loads, and what each one
    contributes to getting from power-on to a running kernel.
    61. the sbi image, the linux kernel, and the dtb image. 
    62. sbi is the mediator, the kernel is the os, dtb is info sbi needs to do its
        mediator job for the os and cpu
61. What CPU/register state has to be correct *before* jumping to
    OpenSBI's entry point for the handoff to work at all?
    62. A0-A2, pc needs to point at open sbi's entry point. 
62. Why does the device tree blob need two specific edits away from
    what QEMU generates, and what would go wrong with each one left
    unedited?
    63. so SBI knows where to find them. if they didn't exist, SBI wouldn't be able
        to pull the needed info.
    64. it needs earlycon, or no visible output is printed out
    65. sstc causes an illegal instruction trap. kernel can't recover. 
63. What's `earlycon` for, and why was it essential for debugging
    early boot failures specifically (as opposed to failures deeper
    into Linux's own boot)?
    64. it bypasses the normal driver probing path. it writes directly
        to uart's known mmio register. writes from the earliest possible
        point in boot. it's for early failures before the console driver 
        has initialized. any panic prior would produce invisible output. 
    65. it allows for debugging before the console printing has fully loaded
64. Why does this project's peripheral and boot design deliberately
    copy QEMU's `virt` machine instead of inventing its own machine
    layout? What two concrete capabilities does that decision buy,
    beyond convenience?
    65. because qemu already matches our desired implementation behavior. 
        it makes testing for correctness reliable. 
    66. cross check emulator behavior. 
    67. osbi's platform driver recognizes this layout. 

## 12. Building the boot files (OpenSBI, kernel, musl, busybox)

65. Why can't OpenSBI be built with the same bare-metal GCC toolchain
    used elsewhere in this project?
    66. osbi needs to produce a pie (position-independent) firmware image.
    67. bare-metal toolchains don't support pie. 
66. Why does the kernel need a specific older tag instead of the tip
    of `master`?
    67. master had a bug. HAVE_GENERIC_VDSO gating doesn't properly exclude 32bit 
        in all code paths, causing undeclared symbol build failures. 
67. Of the kernel config options that get disabled, name at least
    three, and for each say whether it was disabled *because this
    emulator can't decode/handle it* versus *because the hardware it
    models doesn't exist here* versus *because of an unrelated
    toolchain bug*. Those are different reasons; don't collapse them.
    68. emulator doesn't handle CONFIG_FPU.
        emulator has no floating point unit at all
    69. CONFIG_PCI - modeled hardware doesn't exist. these drivers actively
        probe real, memory-mapped registers this emulator has not implemented. 
    70. CONFIG_ATA - has specific driver bugs
68. Why did `F` (floating point) declared in the riscv-arch-test UDB
    config once break booting even for test suites that have nothing
    to do with floating point?
    69. F caused shared boot code to touch fcsr, which caused unhandled trap
        errors that stuck pc at 0. 
69. What problem does the `musl-gcc.specs.sh`-generated specs file
    solve, and what goes wrong if you skip it?
    70. the gnu-gcc is a glibc cross-toolchain by default. its search paths point at 
        glibc. the spec file redirects those paths so a normal compile invocation 
        actually links against musl instead. 
70. Why does `CC` have to be passed on busybox's `make` command line
    instead of being exported as an environment variable?
    71. busybox's makefile does CC = $(CROSS_COMPILE)gcc
        this overrides an exported env variable of the same
        name under make's precedence rules. exported CC gets silently ignored
        as a consequence. 
    72. the build then falls back to a bare compiler with no specs, linking
        against glibc by mistake
71. Why does the initramfs need a hand-written device-node file
    (`nod` entries) at all, instead of just letting the kernel's
    `devtmpfs` create `/dev/console` on its own?
    72. the kernel's pre-init console open attempt needs /dev/console to already
        exist. devtmpfs isn't mounted until /init runs. 
    73. chicken and egg problem, initramfs has to ship the device node itself up 
        front. 
72. Once the `C` extension existed, which specific ISA-narrowing
    decisions became unnecessary, and which ones stayed exactly as
    narrow as before regardless? For each one that stayed narrow,
    explain why `C` support didn't change the reasoning.
    73. RISCV_ISA no longer needed c excluded. f/d stayed narrow. 

## 13. Testing infrastructure

73. Name this project's three layers of correctness testing, and for
    each one, describe the *kind* of bug it's specifically good at
    catching that the others aren't.
    74. unit test, riscv test suite, os boot
    75. unit tests close correctness
    76. test-suite tests in an integration test style
    77. os allows full range testing to pick up anything else not caught
74. What is the `tohost`/`fromhost` protocol, and how does a test
    binary communicate pass/fail using it?
    75. it's where pass/fail information is stored. it stores the test data there.
    76. testnum=1 for pass, odd-encoded value for fail
75. Walk through what `arch_test_runner`'s main loop does, start to
    finish, including the character-output special case.
    76. it loads the test in. cycles until complete, and looks at the tohost value
    77. check if tohost's high word is 0x0101_0000
    78. this means (device=1, cmd=1)
    79. if so, it's a character print request, so we print the byte, clear tohost,
        and keep looking
76. Why does `build.rs` generating one `#[test]` per fixture file work
    better here than one hand-written test function per instruction?
    77. saves writing duplicate test logic
77-80. <REMOVED>


## 14. Putting it together

81. If someone deleted this entire repository and handed you only this
    exam and your own memory, what is the *first* subsystem you'd
    implement, and why does the order matter?
    82. the order is cpu -> tests -> peripherals -> kernel
82. Name the single hardest bug in this project's history and explain
    it end to end.
    83. the in trap nested trap bug.
83. What's still explicitly unfinished or acknowledged as a wart in
    this codebase right now, and why was it left that way rather than
    fixed immediately?
    84. refactor
