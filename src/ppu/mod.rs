#[cfg(target_family = "wasm")]
pub mod emscripten;
pub mod fb;
pub(crate) mod memory;

use std::thread;
use std::sync::mpsc::{channel, Receiver};

use memory::VRAM;

struct MemoryWriteLog {
    address: u16,
    value: u8
}

pub struct PPU {
    vram: VRAM,
}


impl PPU {
    pub fn init_register_handler(&'static mut self, cpu_channel_rx: Receiver<MemoryWriteLog>) {
        thread::spawn(move || {
            for log in cpu_ channel_rx {
                match log.address {
                    0x2000 => self.ppu_ctrl(log.value),
                    _ => {}
                }
            }
        });
    }
    pub fn new() -> Self {
        Self {
            vram: VRAM::new(),
        }
    }

    pub fn ppu_ctrl(&mut self, value: u8){
        
    }

    

}