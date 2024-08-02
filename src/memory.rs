use anyhow::Result;
use simplelog::*;
use std::{fs::File, io::Read};

// Memory abstraction layer, acts as the data and address bus
/// 16-bit address bus
/// Special notes:
/// - $0000-$00FF reserved "zero page"
/// - $0100-$01FF reserved for stack
/// - $FFFA to $FFFF reserved
/// - Little endian
// Includes stack abstraction methods
pub struct Memory {
    pub buffer: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            buffer: vec![0; 0xFFFF + 1],
        }
    }

    pub(crate) fn load_ines_rom(&mut self, path: &str) -> Result<()> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        info!("Loaded {} bytes from ROM", buffer.len());

        let prg_rom_size = buffer[5];
        info!("Program ROM size: {} kb", prg_rom_size * 16);

        let prg_rom_size: usize = (prg_rom_size as usize * 16384).into();
        info!("Copying {} bytes", prg_rom_size);

        let mapper_flags = buffer[7] >> 4;
        info!("Mapper type: {}", mapper_flags);

        info!("RAM size {}", self.buffer.len());

        let prg_rom = &buffer[16..(16 + prg_rom_size)];

        // implementing NROM mapper (mapper 0) for now
        // copy prg-rom to 0x8000 and 0xC000
        self.buffer[0x8000..(0x8000 + prg_rom_size)].clone_from_slice(prg_rom);
        self.buffer[0xC000..(0xC000 + prg_rom_size)].clone_from_slice(prg_rom);

        Ok(())
    }

    pub(crate) fn fetch_absolute(&self, address: u16) -> u8 {
        self.buffer[address as usize]
    }

    pub(crate) fn store_absolute(&mut self, address: u16, value: u8) {
        self.buffer[address as usize] = value
    }

    // also called for absolute_y
    pub(crate) fn fetch_absolute_x(&self, address: u16, index_x: u8) -> u8 {
        self.fetch_absolute(address + (index_x as u16))
    }

    // also called for absolute_y
    pub(crate) fn store_absolute_x(&mut self, address: u16, index_x: u8, value: u8) {
        self.store_absolute(address + (index_x as u16), value)
    }

    pub(crate) fn fetch_zero_page(&self, addr_lower_byte: u8) -> u8 {
        self.fetch_absolute(addr_lower_byte as u16)
    }

    pub(crate) fn store_zero_page(&mut self, addr_lower_byte: u8, value: u8) {
        self.store_absolute(addr_lower_byte as u16, value)
    }

    // also called for zero_page_y
    pub(crate) fn fetch_zero_page_x(&self, addr_lower_byte: u8, index_x: u8) -> u8 {
        let addr = addr_lower_byte.wrapping_add(index_x);
        self.fetch_zero_page(addr)
    }

    pub(crate) fn store_zero_page_x(&mut self, addr_lower_byte: u8, x: u8, value: u8) {
        let addr = addr_lower_byte.wrapping_add(x);
        self.store_absolute(addr as u16, value);
    }

    pub(crate) fn fetch_indirect(&self, address: u16) -> u16 {
        (self.fetch_absolute(address) as u16 + (self.fetch_absolute(address + 1) as u16) * 256)
            as u16
    }

    pub(crate) fn fetch_indirect_x(&self, addr_lower_byte: u8, index_x: u8) -> u8 {
        // val = PEEK(PEEK((arg + X) % 256) + PEEK((arg + X + 1) % 256) * 256)
        let addr = self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x)) as u16
            + self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x + 1)) as u16 * 256;
        self.fetch_absolute(addr)
    }

    pub(crate) fn store_indirect_x(&mut self, addr_lower_byte: u8, index_x: u8, value: u8) {
        let addr = self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x)) as u16
            + self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x + 1)) as u16 * 256;
        self.store_absolute(addr, value)
    }

    pub(crate) fn fetch_indirect_y(&self, addr_lower_byte: u8, index_y: u8) -> u8 {
        // val = PEEK(PEEK(arg) + PEEK((arg + 1) % 256) * 256 + Y)
        let addr = self.fetch_zero_page(addr_lower_byte) as u16
            + self.fetch_zero_page(addr_lower_byte.wrapping_add(1)) as u16 * 256
            + index_y as u16;
        self.fetch_absolute(addr)
    }

    pub(crate) fn store_indirect_y(&mut self, addr_lower_byte: u8, index_y: u8, value: u8) {
        // val = PEEK(PEEK(arg) + PEEK((arg + 1) % 256) * 256 + Y)
        let addr = self.fetch_zero_page(addr_lower_byte) as u16
            + self.fetch_zero_page(addr_lower_byte.wrapping_add(1)) as u16 * 256
            + index_y as u16;
        self.store_absolute(addr as u16, value);
    }

    /*
     * Returns count number of bytes starting from address in memory
     * 
     * Parameters: address to start from, count of bytes to return
     * Return: vector of bytes
     */
    pub(crate) fn fetch_bytes(&self, address: u16, count: usize) -> Vec<u8> {

        (0..count)
        .map(|i| self.buffer[(address as usize + i) % 65536]) 
        .collect()
    }
}
