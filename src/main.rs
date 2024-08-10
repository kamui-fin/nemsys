/// xines - MOS 6502 instruction set implementation
/// Clock speed: 1.789773 MHz

#[macro_use]
extern crate log;
extern crate simplelog;

use jsontest::{CpuTestState, InstructionTestCase, MemTest};
use std::fs::File;
use std::io::Write;
use std::panic;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Result};
use cpu::Cpu;
use simplelog::*;

mod cpu;
mod jsontest;
mod memory;
mod registers;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "nemsys")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Test {
        #[command(subcommand)]
        subcommand: TestSubcommand,
    },
}

#[derive(Subcommand)]
enum TestSubcommand {
    Nestest,
    Singlestep,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Test { subcommand } => match subcommand {
            TestSubcommand::Nestest => run_nestest(),
            TestSubcommand::Singlestep => run_single_step_tests(),
        },
    }
}

fn run_nestest() -> Result<()> {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create("nemsys.log").unwrap(),
        ),
    ])
    .unwrap();

    let mut cpu = Cpu::new();
    let mem = &mut cpu.memory;

    mem.load_ines_rom("nestest/nestest.nes")?;
    cpu.init_pc();

    let start_time = SystemTime::now();

    let target_period = (1.0 / (1.789773 * 1e6)) * 1e9;

    while cpu.num_cycles < 270_000 {
        cpu.tick();
        cpu.memory.databus_logger.clear();

        let actual_period =
            (start_time.elapsed().unwrap().as_nanos() as f64) / (cpu.num_cycles as f64);
        let wait_time = Duration::from_nanos((target_period - actual_period) as u64);

        sleep(wait_time);
    }

    Ok(())
}

fn run_single_step_tests() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Error,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    let all_tests = jsontest::load_json_tests("nes6502/v1")?.enumerate();

    for (i, case_set) in all_tests {
        let num_cases = case_set.test_cases.len();
        for case in case_set.test_cases {
            let result = panic::catch_unwind(|| {
                test_instruction(case.clone());
            });
            if result.is_err() {
                error!("{:#?}", case);
                println!("{:x}.................... [FAILED]", case_set.opcode);
                println!("Passed {}/{} test cases", i + 1, num_cases);
                return Err(anyhow!(case.name));
            }
        }

        println!("{:x}.................... [PASSED]", case_set.opcode);
        let mut checkpoint_file = File::create("/tmp/nemsys.ck").unwrap();
        writeln!(checkpoint_file, "{:x}", case_set.opcode).unwrap();
    }

    Ok(())
}

fn init_cpu_test_state(state: CpuTestState, cpu: &mut Cpu) {
    cpu.registers.stack_pointer = state.s;
    cpu.registers.accumulator = state.a;
    cpu.registers.index_x = state.x;
    cpu.registers.index_y = state.y;
    cpu.registers.processor_status = state.p;
    cpu.registers.program_counter = state.pc;

    for MemTest(address, value) in state.ram {
        cpu.memory.store_absolute(address, value);
    }
}

fn assert_cpu_test_state(state: CpuTestState, cpu: &Cpu) {
    assert_eq!(cpu.registers.stack_pointer, state.s);
    assert_eq!(cpu.registers.accumulator, state.a);
    assert_eq!(cpu.registers.index_x, state.x);
    assert_eq!(cpu.registers.index_y, state.y);
    assert_eq!(
        cpu.registers.processor_status | 0b0010_0000,
        state.p | 0b0010_0000
    );
    assert_eq!(cpu.registers.program_counter, state.pc);

    for MemTest(address, value) in state.ram {
        assert_eq!(cpu.memory.buffer[address as usize], value);
    }
}

fn test_instruction(case: InstructionTestCase) {
    let mut cpu = Cpu::new();

    let initial_state = case.initial;
    init_cpu_test_state(initial_state.clone(), &mut cpu);

    cpu.tick();

    let final_state = case.r#final;
    assert_cpu_test_state(final_state, &cpu); // assert after
                                              // assert_eq!(cpu.memory.databus_logger.log, case.cycles);
}
