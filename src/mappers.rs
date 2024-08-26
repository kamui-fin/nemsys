use std::{fs::File, io::Read};

use anyhow::Result;
use log::info;

use crate::{
    cpu::memory::Memory,
    ppu::{memory::VRAM, NametableArrangement},
};

pub trait Mapper {
    fn from_ines_rom(path: &str, vram: &mut VRAM, wram: &mut Memory) -> Result<Self>
    where
        Self: Sized;
}

pub struct NROM {
    nt_arrangement: NametableArrangement,
}

impl Mapper for NROM {
    fn from_ines_rom(path: &str, vram: &mut VRAM, wram: &mut Memory) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        info!("Loaded {} bytes from ROM", buffer.len());

        let mapper_flags = buffer[7] >> 4;
        info!("Mapper type: {}", mapper_flags);

        let prg_rom_size = buffer[4];
        info!("Program ROM size: {} kb", prg_rom_size * 16);

        let prg_rom_size: usize = prg_rom_size as usize * 16384;
        info!("Copying {} bytes", prg_rom_size);

        let prg_rom = &buffer[16..(16 + prg_rom_size)];

        // implementing NROM mapper (mapper 0) for now
        // copy prg-rom to 0x8000 and 0xC000
        wram.buffer[0x8000..(0x8000 + prg_rom_size)].clone_from_slice(prg_rom);
        wram.buffer[0xC000..(0xC000 + prg_rom_size)].clone_from_slice(prg_rom);

        let nt_arrangement = if buffer[6] & 1 == 0 {
            NametableArrangement::HorizontalMirror
        } else {
            NametableArrangement::VerticalMirror
        };

        let chr_rom_size = buffer[5];
        let chr_rom_size: usize = chr_rom_size as usize * 8192;

        let chr_rom = &buffer[(16 + prg_rom_size)..((16 + prg_rom_size) + chr_rom_size)];
        vram.buffer[0x0000..(0x0000 + chr_rom_size)].clone_from_slice(chr_rom);

        Ok(Self { nt_arrangement })
    }
}
