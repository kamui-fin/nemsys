/// xines - MOS 6502 instruction set implementation
/// Clock speed: 1.789773 MHz

#[macro_use]
extern crate log;
extern crate simplelog;

use std::fs::File;
use std::io::{self, Write};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use cpu::Cpu;
use simplelog::*;

mod cpu;
mod jsontest;
mod memory;
mod registers;

fn main() -> Result<()> {
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

    loop {
        cpu.tick();
        cpu.memory.databus_logger.clear();

        let actual_period =
            (start_time.elapsed().unwrap().as_nanos() as f64) / (cpu.num_cycles as f64);
        let wait_time = Duration::from_nanos((target_period - actual_period) as u64);

        sleep(wait_time);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::panic;

    use jsontest::{CpuTestState, InstructionTestCase, MemTest};

    use super::*;

    #[test]
    fn test_cpu() {
        CombinedLogger::init(vec![TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )])
        .unwrap();

        let all_tests = jsontest::load_json_tests("nes6502/v1").unwrap();

        let iter = all_tests.iter().enumerate();
        for (i, case) in iter {
            let result = panic::catch_unwind(|| {
                test_instruction(case.clone());
            });
            let mut checkpoint_file = File::create("/tmp/nemsys.ck").unwrap();
            writeln!(
                checkpoint_file,
                "{}",
                case.name.split_whitespace().collect::<Vec<&str>>()[0]
            )
            .unwrap();
            if let Err(_) = result {
                error!("Passed {}/{} test cases", i + 1, all_tests.len());
                error!("{:#?}", case);

                panic!();
            }
        }
    }

    fn init_cpu_test_state(state: CpuTestState, cpu: &mut Cpu) {
        cpu.registers.stack_pointer = state.s;
        cpu.registers.accumulator = state.a;
        cpu.registers.index_x = state.x;
        cpu.registers.index_y = state.y;
        cpu.registers.processor_status = state.p;

        for MemTest(address, value) in state.ram {
            cpu.memory.store_absolute(address, value);
        }
    }

    fn assert_cpu_test_state(state: CpuTestState, cpu: &Cpu) {
        assert_eq!(cpu.registers.stack_pointer, state.s);
        assert_eq!(cpu.registers.accumulator, state.a);
        assert_eq!(cpu.registers.index_x, state.x);
        assert_eq!(cpu.registers.index_y, state.y);
        assert_eq!(cpu.registers.processor_status, state.p);

        for MemTest(address, value) in state.ram {
            assert_eq!(cpu.memory.buffer[address as usize], value);
        }
    }

    fn test_instruction(case: InstructionTestCase) {
        let mut cpu = Cpu::new();
        cpu.init_pc(); // set to 0xC000

        let mem = &mut cpu.memory;

        let ins_data: Vec<u8> = case
            .name
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect();

        mem.buffer[0xC000..(0xC000 + ins_data.len())].copy_from_slice(&ins_data);

        init_cpu_test_state(case.initial, &mut cpu);
        cpu.tick();

        let final_state = case.r#final;

        assert_cpu_test_state(final_state, &cpu); // assert after
                                                  // assert_eq!(cpu.memory.databus_logger.log, case.cycles);
    }
}
