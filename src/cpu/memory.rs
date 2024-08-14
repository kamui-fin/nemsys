use anyhow::Result;
use log::info;
use std::{fs::File, io::Read};

use crate::cpu::jsontest::DatabusLog;

pub struct DatabusLogger {
    pub log: Vec<DatabusLog>,
}

impl DatabusLogger {
    pub fn new() -> Self {
        Self { log: vec![] }
    }

    pub fn log_read(&mut self, address: u16, value: u8) {
        self.log
            .push(DatabusLog(address, value, "read".to_string()))
    }

    pub fn log_write(&mut self, address: u16, value: u8) {
        self.log
            .push(DatabusLog(address, value, "write".to_string()))
    }

    pub fn clear(&mut self) {
        self.log = vec![];
    }
}

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
    pub databus_logger: DatabusLogger,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            buffer: vec![0; 0xFFFF + 1],
            databus_logger: DatabusLogger::new(),
        }
    }

    pub fn load_ines_rom(&mut self, path: &str) -> Result<()> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;

        info!("Loaded {} bytes from ROM", buffer.len());

        let prg_rom_size = buffer[5];
        info!("Program ROM size: {} kb", prg_rom_size * 16);

        let prg_rom_size: usize = prg_rom_size as usize * 16384;
        info!("Copying {} bytes", prg_rom_size);

        let mapper_flags = buffer[7] >> 4;
        info!("Mapper type: {}", mapper_flags);

        let prg_rom = &buffer[16..(16 + prg_rom_size)];

        // implementing NROM mapper (mapper 0) for now
        // copy prg-rom to 0x8000 and 0xC000
        self.buffer[0x8000..(0x8000 + prg_rom_size)].clone_from_slice(prg_rom);
        self.buffer[0xC000..(0xC000 + prg_rom_size)].clone_from_slice(prg_rom);

        // panic!();
        Ok(())
    }

    pub fn fetch_absolute(&mut self, address: u16) -> u8 {
        let value = self.buffer[address as usize];
        self.databus_logger.log_read(address, value);

        value
    }

    pub fn store_absolute(&mut self, address: u16, value: u8) {
        self.databus_logger.log_write(address, value);
        self.buffer[address as usize] = value
    }

    // also called for absolute_y
    pub fn fetch_absolute_x(&mut self, address: u16, index_x: u8) -> u8 {
        let address = address.wrapping_add(index_x as u16);
        self.fetch_absolute(address)
    }

    // also called for absolute_y
    pub fn store_absolute_x(&mut self, address: u16, index_x: u8, value: u8) {
        let address = address.wrapping_add(index_x as u16);
        self.store_absolute(address, value)
    }

    pub fn fetch_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let address = addr_lower_byte as u16;
        self.fetch_absolute(address)
    }

    pub fn store_zero_page(&mut self, addr_lower_byte: u8, value: u8) {
        let address = addr_lower_byte as u16;
        self.store_absolute(address, value)
    }

    // also called for zero_page_y
    pub fn fetch_zero_page_x(&mut self, addr_lower_byte: u8, index_x: u8) -> u8 {
        let address = addr_lower_byte.wrapping_add(index_x);
        self.fetch_zero_page(address)
    }

    pub fn store_zero_page_x(&mut self, addr_lower_byte: u8, x: u8, value: u8) {
        let address = addr_lower_byte.wrapping_add(x);
        self.store_absolute(address as u16, value);
    }

    pub fn fetch_indirect_quirk(&mut self, address: u16) -> u16 {
        let next_address = ((address >> 8) << 8) | ((address & 0xFF) as u8).wrapping_add(1) as u16;
        self.fetch_absolute(address) as u16 + (self.fetch_absolute(next_address) as u16) * 256
    }

    pub fn fetch_indirect_x(&mut self, addr_lower_byte: u8, index_x: u8) -> u8 {
        // val = PEEK(PEEK((arg + X) % 256) + PEEK((arg + X + 1) % 256) * 256)
        let addr = self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x)) as u16
            + self.fetch_zero_page(addr_lower_byte.wrapping_add((index_x).wrapping_add(1))) as u16
                * 256;
        self.fetch_absolute(addr)
    }

    pub fn store_indirect_x(&mut self, addr_lower_byte: u8, index_x: u8, value: u8) {
        let addr = self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x)) as u16
            + self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x).wrapping_add(1)) as u16
                * 256;
        self.store_absolute(addr, value)
    }

    pub fn store_indirect_y(&mut self, addr_lower_byte: u8, index_y: u8, value: u8) {
        let addr = self.fetch_zero_page(addr_lower_byte) as u16;
        let addr =
            addr.wrapping_add(self.fetch_zero_page(addr_lower_byte.wrapping_add(1)) as u16 * 256);
        let addr = addr.wrapping_add(index_y as u16);
        self.store_absolute(addr, value);
    }

    pub fn fetch_indirect_y(&mut self, addr_lower_byte: u8, index_y: u8) -> u8 {
        // val = PEEK(PEEK(arg) + PEEK((arg + 1) % 256) * 256 + Y)
        let addr = self.fetch_zero_page(addr_lower_byte) as u16;
        let addr =
            addr.wrapping_add(self.fetch_zero_page(addr_lower_byte.wrapping_add(1)) as u16 * 256);
        let addr = addr.wrapping_add(index_y as u16);
        self.fetch_absolute(addr)
    }
}
