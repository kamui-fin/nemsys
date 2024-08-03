/// xines - MOS 6502 instruction set implementation
/// Clock speed: 1.789773 MHz

#[macro_use]
extern crate log;
extern crate simplelog;

use std::thread::sleep;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use cpu::Cpu;
use simplelog::*;

mod cpu;
mod memory;
mod registers;

fn main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    let mut cpu = Cpu::new();
    let mem = &mut cpu.memory;

    mem.load_ines_rom("romtest/nestest.nes")?;
    cpu.init_pc();

    let start_time = SystemTime::now();

    let target_period = (1.0 / (1.789773 * 1e6)) * 1e9;

    let mut num_cycles: usize = 0;

    loop {
        num_cycles += cpu.tick();
        let actual_period = (start_time.elapsed().unwrap().as_nanos() as f64) / (num_cycles as f64);
        let wait_time = Duration::from_nanos((target_period - actual_period) as u64);

        sleep(wait_time);
    }
}
