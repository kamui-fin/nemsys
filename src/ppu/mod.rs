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
        let base_nametable_address: usize = match value & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => 0x0000 // will never hit
        };
        let increment: usize = if value & (1 << 2) > 0 {1} else {32};
        let sprite_pattern_address: usize = if value & (1 << 3) > 0 { 0x1000 } else { 0x0000 };
        let bg_pattern_address: usize = if value & (1 << 4) > 0 { 0x0000 } else { 0x1000 };
        let mode = (value & (1 << 5)) >> 5; // 0 for 8x8, 1 for 8x16
        let master_slave_select = (value & (1 << 6)) >> 6; // (0: read backdrop from EXT pins; 1: output color on EXT pins)
        let generate_nmi = (value & (1 << 7)) >> 7; // Generate an NMI at the start of the vertical blanking interval (0: off; 1: on)
    }
}