use anyhow::Result;
use log::info;
use ppu::memory::VRAM;
use sdl2::keyboard::Keycode;
use std::{
    cell::RefCell,
    fs::File,
    io::Read,
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
};

use crate::{
    cpu::jsontest::DatabusLog,
    ppu::{self, PPU},
    utils::{set_bit, unset_bit},
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
pub struct Memory {
    pub buffer: Vec<u8>,
    pub databus_logger: DatabusLogger,
    pub ppu: Rc<RefCell<PPU>>,
    pub input: KeyboardController,
}

impl Memory {
    pub fn new(ppu: Rc<RefCell<PPU>>) -> Self {
        Self {
            buffer: vec![0; 0xFFFF + 1],
            databus_logger: DatabusLogger::new(),
            input: KeyboardController::new(),
            ppu,
        }
    }

    pub fn fetch_absolute(&mut self, address: u16) -> u8 {
        let value = self.buffer[address as usize];
        // self.databus_logger.log_read(address, value);
        match address {
            0x2002 => self.ppu.borrow_mut().ppu_status(),
            0x2004 => self.ppu.borrow_mut().oam_data_read(),
            0x2007 => self.ppu.borrow_mut().ppu_data_read(),
            0x4016 => self.input.read_controller_one(),
            _ => value,
        }
    }

    pub fn store_absolute(&mut self, address: u16, value: u8) {
        // self.databus_logger.log_write(address, value);
        match address {
            0x2000 => self.ppu.borrow_mut().ppu_ctrl(value),
            0x2001 => self.ppu.borrow_mut().ppu_mask(value),
            0x2003 => self.ppu.borrow_mut().oam_addr(value),
            0x2004 => self.ppu.borrow_mut().oam_data_write(value),
            0x2005 => self.ppu.borrow_mut().ppu_scroll(value),
            0x2006 => self.ppu.borrow_mut().ppu_addr(value),
            0x2007 => self.ppu.borrow_mut().ppu_data_write(value),
            0x4014 => self.ppu.borrow_mut().oam_dma(
                &self.buffer
                    [(((value as u16) << 8) as usize)..=((((value as u16) << 8) | 0xFF) as usize)],
            ),
            0x4016 => self.input.write_register(value),
            _ => {}
        };
        self.buffer[address as usize] = value;
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

// ---- INPUT ----
// 0 - A - a
// 1 - B - s
// 2 - Select - (
// 3 - Start - )
// 4 - Up
// 5 - Down
// 6 - Left
// 7 - Right

// Input ($4016 write)
// Output ($4016/$4017 read)

pub struct KeyboardController {
    pub strobe_activated: bool,
    button_register: u8,
    button_latch: u8,
    read_count: usize,
}

impl KeyboardController {
    pub fn new() -> Self {
        Self {
            strobe_activated: false,
            button_register: 0b1111_1111,
            button_latch: 0b1111_1111,
            read_count: 0,
        }
    }

    pub fn handle_keypress(&mut self, key: Keycode) {
        // unset bit
        match key {
            Keycode::A => self.button_latch = unset_bit(self.button_latch.into(), 0),
            Keycode::S => self.button_latch = unset_bit(self.button_latch.into(), 1),
            Keycode::MINUS => self.button_latch = unset_bit(self.button_latch.into(), 2),
            Keycode::EQUALS => self.button_latch = unset_bit(self.button_latch.into(), 3),
            Keycode::UP => self.button_latch = unset_bit(self.button_latch.into(), 4),
            Keycode::DOWN => self.button_latch = unset_bit(self.button_latch.into(), 5),
            Keycode::LEFT => self.button_latch = unset_bit(self.button_latch.into(), 6),
            Keycode::RIGHT => self.button_latch = unset_bit(self.button_latch.into(), 7),
            _ => {}
        }

        self.latch();

        // println!(
        //     "Key: {} Latch {:#010b}, Reg {:#010b}, strobe: {}",
        //     key.name(),
        //     self.button_latch,
        //     self.button_register,
        //     self.strobe_activated
        // );
    }

    pub fn handle_release(&mut self, key: Keycode) {
        // set bit
        match key {
            Keycode::A => self.button_latch = set_bit(self.button_latch.into(), 0),
            Keycode::S => self.button_latch = set_bit(self.button_latch.into(), 1),
            Keycode::MINUS => self.button_latch = set_bit(self.button_latch.into(), 2),
            Keycode::EQUALS => self.button_latch = set_bit(self.button_latch.into(), 3),
            Keycode::UP => self.button_latch = set_bit(self.button_latch.into(), 4),
            Keycode::DOWN => self.button_latch = set_bit(self.button_latch.into(), 5),
            Keycode::LEFT => self.button_latch = set_bit(self.button_latch.into(), 6),
            Keycode::RIGHT => self.button_latch = set_bit(self.button_latch.into(), 7),
            _ => {}
        }

        self.latch();

        // println!(
        //     "Key: {} Latch {:#010b}, Reg {:#010b}, strobe: {}",
        //     key.name(),
        //     self.button_latch,
        //     self.button_register,
        //     self.strobe_activated
        // );
    }

    pub fn latch(&mut self) {
        if self.strobe_activated {
            self.button_register = self.button_latch;
        }
    }

    pub fn write_register(&mut self, value: u8) {
        // println!("Writing {value} to strobe");
        if value == 1 {
            // reloading shift registers with new input data
            self.read_count = 0;
            self.strobe_activated = true;
            self.latch();
        } else {
            self.strobe_activated = false;
        }
    }

    pub fn read_controller_one(&mut self) -> u8 {
        // println!(
        //     "Read: {:#010b} count {}, strobe {}",
        //     self.button_register, self.read_count, self.strobe_activated
        // );
        if self.read_count >= 8 {
            return 0b1111_1111;
        }

        self.read_count += 1;

        let curr_bit = self.button_register & 1;
        self.button_register = self.button_register >> 1;

        curr_bit
    }
}
