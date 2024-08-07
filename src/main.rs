/// xines - MOS 6502 instruction set implementation
/// Clock speed: 1.789773 MHz

#[macro_use]
extern crate log;
extern crate simplelog;

use std::fs::File;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use cpu::Cpu;
use simplelog::*;

mod registers;
mod cpu;
mod memory;

fn main() -> Result<()> {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ), 
        WriteLogger::new(LevelFilter::Info, Config::default(), File::create("xines.log").unwrap())
    ])
    .unwrap();

    let mut cpu = Cpu::new();
    let mem = &mut cpu.memory;

    mem.load_ines_rom("romtest/nestest.nes")?;
    cpu.init_pc();

    let start_time = SystemTime::now();

    let target_period = (1.0 / (1.789773 * 1e6)) * 1e9;

    loop {
        cpu.tick();
        let actual_period =
            (start_time.elapsed().unwrap().as_nanos() as f64) / (cpu.num_cycles as f64);
        let wait_time = Duration::from_nanos((target_period - actual_period) as u64);

        sleep(wait_time);

        if cpu.num_cycles > 1000 {
            break;
        }
    }
    
    Ok(())
}
