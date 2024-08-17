use std::thread::{self, sleep};
use std::sync::mpsc::channel;
use std::time::Duration;

use log::LevelFilter;
use nemsys::cpu::memory::MemoryAccessLog;
use nemsys::cpu::Cpu;
use nemsys::ppu::PPU;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};


fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
    ])
    .unwrap();
    let mut ppu = PPU::new();
    let mut cpu = Cpu::new(&mut ppu);

    cpu.memory.store_absolute(0x2000, 0x4C);
}