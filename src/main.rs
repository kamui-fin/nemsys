/// xines - MOS 6502 instruction set implementation
/// Clock speed: 1.789773 MHz

#[macro_use]
extern crate log;
extern crate simplelog;

use anyhow::Result;
use cpu::Cpu;
use memory::Memory;
use simplelog::*;

mod cpu;
mod memory;
mod registers;

fn main() -> Result<()> {
    let a = [1, 2, 3, 4, 5];
    let s = &a[3..5];
    assert_eq!(s, &[4, 5]);
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    let mut cpu = Cpu::new();
    let mem = &mut cpu.memory;

    mem.load_ines_rom("romtest/nestest.nes")
    // let running = true;

    // while running {
    //     // cpu.tick();
    //     // sleep for a bit
    // }
}
