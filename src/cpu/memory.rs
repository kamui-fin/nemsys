use anyhow::Result;
use log::info;
use ppu::memory::VRAM;
use std::{fs::File, io::Read, sync::mpsc::Receiver, sync::mpsc::Sender};

use crate::{
    cpu::jsontest::DatabusLog,
    ppu::{self, PPU},
};

// WriteCallback: range -> fn
// pointers into VRAM

// ReadCallback (???)

pub struct MemoryAccessLog {
    pub address: u16,
    pub value: u8,
}

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
pub struct Memory<'s> {
    pub buffer: Vec<u8>,
    pub databus_logger: DatabusLogger,
    pub ppu: &'s mut PPU,
}

impl<'s> Memory<'s> {
    pub fn new(ppu: &'s mut PPU) -> Self {
        Self {
            buffer: vec![0; 0xFFFF + 1],
            databus_logger: DatabusLogger::new(),
            ppu,
        }
    }

    pub fn fetch_absolute(&mut self, address: u16) -> u8 {
        let value = self.buffer[address as usize];
        self.databus_logger.log_read(address, value);
        match address {
            0x2002 => self.ppu.ppu_status(),
            0x2004 => self.ppu.oam_data_read(),
            0x2007 => self.ppu.ppu_data_read(),
            _ => value,
        }
    }

    pub fn store_absolute(&mut self, address: u16, value: u8) {
        self.databus_logger.log_write(address, value);
        match address {
            0x2000 => self.ppu.ppu_ctrl(value),
            0x2001 => self.ppu.ppu_mask(value),
            0x2003 => self.ppu.oam_addr(value),
            0x2004 => self.ppu.oam_data_write(value),
            0x2005 => self.ppu.ppu_scroll(value),
            0x2006 => self.ppu.ppu_addr(value),
            0x2007 => self.ppu.ppu_data_write(value),
            // FIXME:
            // 0x4014 => self.ppu.oam_dma(&self.buffer[((value << 8) as usize)..=(((value << 8) | 0xFF) as usize)]),
            _ => self.buffer[address as usize] = value,
        }
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
