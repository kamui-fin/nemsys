use std::thread::{self, sleep};
use std::sync::mpsc::channel;
use std::time::Duration;

use log::LevelFilter;
use nemsys::cpu::memory::MemoryWriteLog;
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
    let (cpu_channel_tx, cpu_channel_rx) = channel::<MemoryWriteLog>();

    let mut cpu = Cpu::new(cpu_channel_tx);
    let mut ppu = PPU::new();

    thread::spawn(move || {
        for log in cpu_channel_rx {
            match log.address {
                0x2000 => ppu.ppu_ctrl(log.value),
                _ => {}
            }
        }
    });

    cpu.memory.store_absolute(0x2000, 0x4C);
    sleep(Duration::from_millis(100));
}