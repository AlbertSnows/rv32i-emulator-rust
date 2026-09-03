// ## PLIC — Platform-Level Interrupt Controller
//
// **What it is.** A memory-mapped device that multiplexes many external
// interrupt sources (UART RX, virtio devices, etc.) onto each hart's
// single `meip`/`seip` line, with per-source priority and per-context
// enable/threshold/claim registers.
//
// **Why this project needs it.** `mip.MEIP`/`SEIP` (already modeled in
// `csr.rs`) are the *destination* of an external interrupt notification,
// but this emulator has no mechanism to *set* them from a device. PLIC is
// that mechanism — without it, any peripheral beyond a polled UART can
// never actually interrupt the CPU.
//
// **Spec:** [RISC-V PLIC Specification v1.0.0](https://github.com/riscv/riscv-plic-spec/releases/download/1.0.0/riscv-plic-1.0.0.pdf)
// (ratified 2023-03, CC-BY-4.0, 16 pages — short and precise).
//
// ### Section outline
//
// - **§1 Introduction** — the model: gateways convert raw device signals
// into a single pending request per source; the PLIC core holds
// priority/pending/enable state; targets are hart+privilege contexts.
// Up to 1023 interrupt sources (ID 0 reserved = "no interrupt") and
// 15872 contexts, though real implementations declare far fewer.
// - §1.1 Interrupt Targets and Hart Contexts — a "context" is (hart,
// privilege mode); this project needs exactly one context per
// implemented privilege mode per hart (M and S, single hart).
// - §1.2 Interrupt Gateways — level-triggered sources (UART fits this)
// won't raise a second request for the same source until the
// previous one is completed.
// - §1.4 Interrupt Identifiers — IDs start at 1; lower ID wins
// priority ties.
// - **§2 Operation Parameters** — names the six register blocks: Priority,
// Pending, Enable, Threshold, Claim, Completion (claim and completion
// share one register).
// - **§3 Memory Map** — the actual byte layout, all registers 32-bit,
// accessed with `lw`/`sw`:
// - `base + 0x000000`–`0x000FFC`: per-source priority (source 0
// reserved)
// - `base + 0x001000`–`0x00107C`: pending bits, 32 sources/word
// - `base + 0x002000` + `0x80*context`: per-context enable bits
// - `base + 0x200000` + `0x1000*context`: threshold (offset +0) and
// claim/complete (offset +4) per context
// - **§4 Interrupt Priorities** — priority 0 = "never interrupt"; higher
// integer = higher priority; can legally be hardwired if not
// implementing WARL discovery.
// - **§5 Interrupt Pending Bits** — read-only status; bit `(N mod 32)` of
// word `(N/32)`.
// - **§6 Interrupt Enables** — one bit per (source, context); bit 0 of
// context 0's first word is hardwired 0 (source 0 doesn't exist).
// - **§7 Priority Thresholds** — per-context WARL register; masks all
// interrupts at or below the threshold value.
// - **§8 Interrupt Claim Process** — reading the claim/complete register
// atomically returns the highest-priority pending source ID for that
// context *and* clears its pending bit; returns 0 if nothing pending.
// - **§9 Interrupt Completion** — writing the claimed ID back to the same
// register tells the gateway to accept a new request from that source.
//
// ### What's needed (minimal, single hart, few sources)
//
// A source count in the single digits (UART = 1, plus a couple of
// virtio-mmio lines if that comes later) means most of the 1023-source /
// 15872-context address space is unused/reserved space you don't need to
// back with real storage — implement it as "reads as zero, ignore
// out-of-range writes" and only give real backing to the sources and
// contexts you declare. Two contexts (M and S) is enough for one hart.
// Wire the PLIC's per-context EIP output into the existing
// `mip.MEIP`/`SEIP` CSR bits so `step()`'s trap-check logic (whatever
// already polls `mip`) picks it up unchanged.