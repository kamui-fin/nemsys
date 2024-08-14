#[cfg(target_family = "wasm")]
pub mod emscripten;
pub(crate) mod memory;

use std::thread;
use std::sync::mpsc::{channel, Receiver};

use log::info;
use memory::VRAM;

pub struct PPU {
    vram: VRAM,
}


impl PPU {
    pub fn new() -> Self {
        Self {
            vram: VRAM::new(),
        }
    }

    pub fn ppu_ctrl(&mut self, value: u8){
        info!("HIIIIIIIIIIIII");
    }
}