use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;
use std::fs;
use rv32i_emulator::cpu::core::step;
use rv32i_emulator::cpu::definitions::addresses;
use rv32i_emulator::cpu::definitions::codes::ExecutionSignal;
use rv32i_emulator::cpu::definitions::cpu::cpu_definition::{build_cpu_state, CPUMode};
use rv32i_emulator::cpu::definitions::masks;
use rv32i_emulator::loader::boot_kernel;
use rv32i_emulator::peripherals::plic;
use rv32i_emulator::peripherals::uart::UART_SOURCE_ID;
use rv32i_emulator::utility::host_io::read_stdin_loop;

fn load_symbols(path: &str) -> Vec<(usize, String)> {
    let content = fs::read_to_string(path).expect("System.map should exist");
    let mut syms: Vec<(usize, String)> = content
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let addr = usize::from_str_radix(parts.next()?, 16).ok()?;
            let _kind = parts.next()?;
            let name = parts.next()?.to_string();
            Some((addr, name))
        })
        .collect();
    syms.sort_by_key(|(addr, _)| *addr);
    syms
}

fn resolve(syms: &[(usize, String)], pc: usize) -> &str {
    match syms.binary_search_by_key(&pc, |(addr, _)| *addr) {
        Ok(idx) => &syms[idx].1,
        Err(0) => "<before-start>",
        Err(idx) => &syms[idx - 1].1,
    }
}

struct DebugFlags {
    symbols: Option<String>,
    trace_traps: bool,
    trace_plic: bool,
    checkpoint_interval: u64,
    max_steps: u64
}

fn parse_args() -> DebugFlags{
    let mut flags = DebugFlags {
      symbols: None,
        trace_traps: false,
        trace_plic: false,
        checkpoint_interval: 50_000_000,
        max_steps: 3_000_000_000,
    };
    let mut args = std::env::args().skip(1); // argv[0] is binary path, skip
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--symbols" => {
                flags.symbols = Some(args.next().expect("--symbols needs a path"))
            },
            "--trace-traps" => flags.trace_traps = true,
            "--trace-plic" => flags.trace_plic = true,
            "--checkpoint-interval" => {
                let raw = args.next().expect("--checkpoint-interval needs a number");
                let as_num: u64 = raw.parse().expect("must be a number");
                if as_num == 0 {
                    panic!("invalid step arg 0");
                }
                flags.checkpoint_interval = as_num;
            },
            "--max-steps" => {
                let raw = args.next().expect("--max-steps needs a number");
                let as_num = raw.parse().expect("must be a number");
                flags.max_steps = as_num;
            },
            other => panic!("unknown argument: {}", other),
        }
    }
    flags
}

fn main() {
    let flags = parse_args();
    let syms = flags.symbols.as_deref().map(load_symbols);
    let mut cpu = build_cpu_state();
    boot_kernel(&mut cpu).expect("kernel should boot");

    let (tx, rx) = mpsc::channel::<u8>();
    thread::spawn(move || read_stdin_loop(tx));

    let mut last_mode = cpu.mode;
    let mut last_in_trap = cpu.flags.in_trap;
    let mut trap_cause_counts: std::collections::HashMap<(bool, u32), u64> = std::collections::HashMap::new();
    let mut i: u64 = 0;
    let mut total_faults: u64 = 0;
    let mut total_ecalls: u64 = 0;
    let mut distinct_fault_addrs: HashSet<u32> = HashSet::new();
    let mut distinct_u_pcs: HashSet<u32> = HashSet::new();
    let mut fault_addr_hits: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    let mut in_trap_since: Option<u64> = None;
    let mut last_fn = "";

    loop {
        i += 1;
        if let Ok(byte) = rx.try_recv() {
            eprintln!("step {i}: fed byte {:?} into UART", byte as char);
            cpu.bus.receive_uart_byte(byte);
            if flags.trace_plic {
                let priority = cpu.bus.plic.read(plic::PRIORITY_BASE + (UART_SOURCE_ID as u32) * 4, 4);
                let threshold_s = cpu.bus.plic.read(
                    plic::CONTEXT_BASE + plic::CONTEXT_STRIDE * (plic::S_CONTEXT as u32) + plic::THRESHOLD_LOCAL_OFFSET,
                    4,
                );
                let pending_word = cpu.bus.plic.read(plic::PENDING_BASE, 4);
                let pending_uart = (pending_word >> UART_SOURCE_ID) & 1;
                let enabled_s_word = cpu.bus.plic.read(
                    plic::ENABLE_BASE + plic::ENABLE_STRIDE * (plic::S_CONTEXT as u32),
                    4,
                );
                let enabled_uart_s = (enabled_s_word >> UART_SOURCE_ID) & 1;
                let armed = cpu.bus.plic.armed[UART_SOURCE_ID];
                let eip_s = cpu.bus.plic.compute_eip(plic::S_CONTEXT);
                let mie = cpu.csr.read(addresses::MIE, CPUMode::M).unwrap();
                let mip = cpu.csr.read(addresses::MIP, CPUMode::M).unwrap();
                let sstatus = cpu.csr.read(addresses::SSTATUS, CPUMode::M).unwrap();
                let seie = mie & masks::SEIE != 0;
                let seip = mip & masks::SEIP != 0;
                let sie_bit = sstatus & masks::GLOBAL_SIE != 0;
                eprintln!("  PLIC: priority[UART]={} threshold[S]={} pending[UART]={} enabled[S][UART]={} armed[UART]={} compute_eip(S)={} | mie.SEIE={} mip.SEIP={} sstatus.SIE={} mode={:?} in_trap={}",
                          priority, threshold_s, pending_uart, enabled_uart_s, armed, eip_s, seie, seip, sie_bit, cpu.mode, cpu.flags.in_trap);
            }
        }

        let outcome = step(&mut cpu);
        let pc = cpu.pc.read() as u32;

        if cpu.mode == CPUMode::U {
            distinct_u_pcs.insert(pc);
        }

        if cpu.mode == CPUMode::S && last_mode == CPUMode::U {
            let scause = cpu.csr.read(addresses::SCAUSE, CPUMode::M).unwrap();
            let stval = cpu.csr.read(addresses::STVAL, CPUMode::M).unwrap();
            if scause == 8 {
                total_ecalls += 1;
                let a7 = cpu.register.read(17);
                let a0 = cpu.register.read(10);
                eprintln!("step {i}: ecall #{} a7(syscall)={} a0={:#x}", total_ecalls, a7, a0);
            } else if scause == 12 || scause == 13 || scause == 15 {
                total_faults += 1;
                distinct_fault_addrs.insert(stval);
                *fault_addr_hits.entry(stval).or_insert(0) += 1;
            }
        }
        last_mode = cpu.mode;

        if cpu.flags.in_trap && !last_in_trap {
            let is_m = cpu.mode == CPUMode::M;
            let cause = if is_m {
                cpu.csr.read(addresses::MCAUSE, CPUMode::M).unwrap()
            } else {
                cpu.csr.read(addresses::SCAUSE, CPUMode::M).unwrap()
            };
            *trap_cause_counts.entry((is_m, cause)).or_insert(0) += 1;
        }
        if cpu.flags.in_trap && !last_in_trap {
            in_trap_since = Some(i);
        } else if !cpu.flags.in_trap {
            in_trap_since = None;
        }
        if flags.trace_traps {
            if let Some(since) = in_trap_since {
                let elapsed = i - since;
                if elapsed > 0 && elapsed % 1_000_000 == 0 {
                    let f = syms.as_deref().map(|s| resolve(s, pc as usize)).unwrap_or("<no symbols>");
                    if f != last_fn || elapsed % 10_000_000 == 0 {
                        eprintln!("step {i}: STILL in_trap since step {since} ({elapsed} steps) at pc={:#x} ({}) mode={:?}", pc, f, cpu.mode);
                        last_fn = f;
                    }
                }
            }
        }
        last_in_trap = cpu.flags.in_trap;

        if i % flags.checkpoint_interval == 0 {
            eprintln!(
                "step {i}: ecalls={} faults={} distinct_fault_addrs={} distinct_u_pcs={}",
                total_ecalls, total_faults, distinct_fault_addrs.len(), distinct_u_pcs.len()
            );
            let mut top: Vec<(&u32, &u64)> = fault_addr_hits.iter().collect();
            top.sort_by(|a, b| b.1.cmp(a.1));
            for (addr, count) in top.iter().take(5) {
                eprintln!("  hot fault addr {:#x}: {} times", addr, count);
            }
            if flags.trace_traps {
                eprintln!("  trap cause tally:");
                let mut tally: Vec<(&(bool, u32), &u64)> = trap_cause_counts.iter().collect();
                tally.sort_by(|a, b| b.1.cmp(a.1));
                for ((is_m, cause), count) in tally.iter().take(10) {
                    eprintln!("    mode={} cause={:#x}: {} times", if *is_m { "M" } else { "S" }, cause, count);
                }
            }
        }

        if let Ok(ExecutionSignal::Halt) = outcome {
            eprintln!("HALT at step {i}, pc={:#x}", pc);
            break;
        }
        if i > flags.max_steps {
            eprintln!("giving up after {i} steps");
            break;
        }
    }
}
