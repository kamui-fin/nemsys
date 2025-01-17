use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
};

use log::{error, info};
use memory::MemoryAccessLog;

use crate::ppu::PPU;

pub mod jsontest;
pub mod memory;
pub mod registers;

pub struct Cpu {
    pub memory: memory::Memory,
    pub registers: registers::Registers,

    pub num_cycles: usize, // elapsed # of cycles
}

impl Cpu {
    pub fn new(ppu: Rc<RefCell<PPU>>) -> Self {
        Self {
            memory: memory::Memory::new(ppu),
            registers: registers::Registers::new(),
            num_cycles: 0,
        }
    }

    pub fn init_pc(&mut self) {
        self.registers.program_counter = self.fetch_u16(0xFFFC);
        info!("Initialize PC = {:x}", self.registers.program_counter);
    }

    // Helper method
    fn update_zero_negative_flags(&mut self, value: u8) {
        if value == 0 {
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }
        if value & 0b1000_0000 != 0 {
            self.registers.set_neg();
        } else {
            self.registers.unset_neg();
        }
    }

    /*
     * Stack abstraction methods
     */

    fn stack_push(&mut self, val: u8) {
        let stack_addr: u16 = ((0x01_u16) << 8) | self.registers.stack_pointer as u16;
        self.memory.buffer[stack_addr as usize] = val;
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_sub(1);
    }

    fn stack_pop(&mut self) -> u8 {
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_add(1);
        let stack_addr: u16 = ((0x01_u16) << 8) | self.registers.stack_pointer as u16;
        self.memory.buffer[stack_addr as usize]
    }

    /*
     * ADC - Add with Carry
     * This instruction adds the contents of a memory location to the accumulator together with the carry bit. If overflow occurs the carry bit is set, this enables multiple byte addition to be performed.
     */

    // Opcode: $69
    // 2 cycles
    fn adc_immediate(&mut self, value: u8) -> u8 {
        // check if both are positive or if both are negative
        let same_sign = (value & 0b1000_0000) == (self.registers.accumulator & 0b1000_0000);

        let sum: u16 = (self.registers.accumulator as u16)
            + (value as u16)
            + (self.registers.get_carry() as u16);

        // check if two positive sum to neg or vice versa
        if same_sign && ((sum & 0b1000_0000) != (value & 0b1000_0000) as u16) {
            self.registers.set_overflow()
        } else {
            self.registers.unset_overflow();
        }

        // if need to use u16 range, then carry detected
        if sum > 0xFF {
            self.registers.set_carry();
            self.registers.accumulator = (sum & 0b1111_1111) as u8; // TODO: gay code
        } else {
            self.registers.unset_carry();
            self.registers.accumulator = sum as u8;
        }

        self.update_zero_negative_flags(self.registers.accumulator);
        2
    }

    // Opcode: $65
    // 3 cycles
    fn adc_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.adc_immediate(value);

        3
    }

    // Opcode: $75
    // 4 cycles
    fn adc_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.adc_immediate(value);

        4
    }

    // Opcode: $6D
    // 4 cycles
    fn adc_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.adc_immediate(value);

        4
    }

    // Opcode: $7D
    // 4 (+1 if page crossed) cycles
    fn adc_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.adc_immediate(value);

        4
    }

    // Opcode: $79
    // 4 (+1 if page crossed) cycles
    fn adc_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.adc_immediate(value);

        4
    }

    // Opcode: $61
    // 6 cycles
    fn adc_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.adc_immediate(value);

        6
    }

    // Opcode: $71
    // 5 (+1 if page crossed) cycles
    fn adc_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.adc_immediate(value);

        5
    }

    /*
     * SBC - Subtract with Carry
     * This instruction subtracts the contents of a memory location to the accumulator together with the not of the carry bit. If overflow occurs the carry bit is clear, this enables multiple byte subtraction to be performed.
     */

    // Opcode: $E9
    // 2 cycles
    fn sbc_immediate(&mut self, value: u8) -> u8 {
        self.adc_immediate(!value); // twos complement

        2
    }

    // Opcode: $E5
    // 3 cycles
    fn sbc_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.sbc_immediate(value);

        3
    }

    // Opcode: $F5
    // 4 cycles
    fn sbc_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.sbc_immediate(value);

        4
    }

    // Opcode: $ED
    // 4 cycles
    fn sbc_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.sbc_immediate(value);

        4
    }

    // Opcode: $FD
    // 4 (+1 if page crossed) cycles
    fn sbc_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.sbc_immediate(value)
    }

    // Opcode: $F9
    // 4 (+1 if page crossed) cycles
    fn sbc_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.sbc_immediate(value);

        4
    }

    // Opcode: $E1
    // 6 cycles
    fn sbc_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.sbc_immediate(value);

        6
    }

    // Opcode: $F1
    // 5 (+1 if page crossed) cycles
    fn sbc_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.sbc_immediate(value);

        5
    }

    /*
     * CMP - Compare
     * This instruction compares the contents of the accumulator with another memory held value and sets the zero and carry flags as appropriate.
     */

    // Opcode: $C9
    // 2 cycles
    fn cmp_immediate(&mut self, value: u8) -> u8 {
        if self.registers.accumulator == value {
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }
        if self.registers.accumulator >= value {
            self.registers.set_carry();
        } else {
            self.registers.unset_carry();
        }

        let result = self.registers.accumulator.wrapping_sub(value);

        if result & 0b1000_0000 > 0 {
            self.registers.set_neg();
        } else {
            self.registers.unset_neg();
        }

        2
    }

    // Opcode: $E5
    // 3 cycles
    fn cmp_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.cmp_immediate(value);

        3
    }

    // Opcode: $D5
    // 4 cycles
    fn cmp_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.cmp_immediate(value);

        4
    }

    // Opcode: $CD
    // 4 cycles
    fn cmp_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.cmp_immediate(value);

        4
    }

    // Opcode: $DD
    // 4 (+1 if page crossed) cycles
    fn cmp_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.cmp_immediate(value);

        4
    }

    // Opcode: $D9
    // 4 (+1 if page crossed) cycles
    fn cmp_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.cmp_immediate(value);

        4
    }

    // Opcode: $C1
    // 6 cycles
    fn cmp_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.cmp_immediate(value);

        6
    }

    // Opcode: $D1
    // 5 (+1 if page crossed) cycles
    fn cmp_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.cmp_immediate(value);

        5
    }

    /*
     * CPX - Compare X Register
     * This instruction compares the contents of the X register with another memory held value and sets the zero and carry flags as appropriate.
     */

    // Opcode: $E0
    // 2 cycles
    fn cpx_immediate(&mut self, value: u8) -> u8 {
        if self.registers.index_x == value {
            self.registers.set_zero()
        } else {
            self.registers.unset_zero()
        }
        if self.registers.index_x >= value {
            self.registers.set_carry()
        } else {
            self.registers.unset_carry()
        }

        let result = self.registers.index_x.wrapping_sub(value);

        // // POTENTIAL BUG: do we set bit 7 to neg flag directly or only if neg?
        if result & 0b1000_0000 > 0 {
            // if self.registers.index_x < value {
            self.registers.set_neg()
        } else {
            self.registers.unset_neg()
        }

        2
    }

    // Opcode: $E4
    // 3 cycles
    fn cpx_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.cpx_immediate(value);

        3
    }

    // Opcode: $EC
    // 4 cycles
    fn cpx_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.cpx_immediate(value);

        4
    }

    /*
     * CPY - Compare Y Register
     * This instruction compares the contents of the Y register with another memory held value and sets the zero and carry flags as appropriate.
     */

    // Opcode: $C0
    // 2 cycles
    fn cpy_immediate(&mut self, value: u8) -> u8 {
        if self.registers.index_y == value {
            self.registers.set_zero()
        } else {
            self.registers.unset_zero()
        }
        if self.registers.index_y >= value {
            self.registers.set_carry()
        } else {
            self.registers.unset_carry()
        }

        let result = self.registers.index_y.wrapping_sub(value);

        if result & 0b1000_0000 > 0 {
            // if self.registers.index_y < value {
            self.registers.set_neg()
        } else {
            self.registers.unset_neg()
        }

        2
    }

    // Opcode: $C4
    // 3 cycles
    fn cpy_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.cpy_immediate(value);

        3
    }

    // Opcode: $CC
    // 4 cycles
    fn cpy_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.cpy_immediate(value);

        4
    }

    /*
     * ASL - Arithmetic Shift Left
     * This operation shifts all the bits of the accumulator or memory contents one bit left. Bit 0 is set to 0 and bit 7 is placed in the carry flag. The effect of this operation is to multiply the memory contents by 2 (ignoring 2's complement considerations), setting the carry if the result will not fit in 8 bits.
     */

    // Helper method to extract general ASL functionality
    fn asl_immediate(&mut self, value: u8) -> u8 {
        // Bit 7 is set in carry flag
        let first_bit = value & 0b1000_0000;
        if first_bit > 0 {
            self.registers.set_carry()
        } else {
            self.registers.unset_carry()
        }

        let new_value = value << 1;

        self.update_zero_negative_flags(new_value);

        new_value
    }

    // Opcode: $0A
    // 2 cycles
    fn asl_accumulator(&mut self) -> u8 {
        let new_accum = self.asl_immediate(self.registers.accumulator);
        self.registers.accumulator = new_accum;

        2
    }

    // Opcode: $06
    // 5 cycles
    fn asl_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.asl_immediate(value);
        self.memory.store_zero_page(addr_lower_byte, value);

        5
    }

    // Opcode: $16
    // 6 cycles
    fn asl_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.asl_immediate(value);
        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, value);

        6
    }

    // Opcode: $0E
    // 6 cycles
    fn asl_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        let value = self.asl_immediate(value);
        self.memory.store_absolute(address, value);

        6
    }

    // Opcode: $1E
    // 7 cycles
    fn asl_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.asl_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_x, value);

        7
    }

    fn asl_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        let value = self.asl_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_y, value);
    }

    fn asl_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        let value = self.asl_immediate(value);
        self.memory
            .store_indirect_x(addr_lower_byte, self.registers.index_x, value);
    }

    fn asl_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        let value = self.asl_immediate(value);
        self.memory
            .store_indirect_y(addr_lower_byte, self.registers.index_y, value);
    }

    /*
     * LSR - Logical Shift Right
     * Each of the bits in A or M is shift one place to the right. The bit that was in bit 0 is shifted into the carry flag. Bit 7 is set to zero.
     */

    // Helper method to extract general ASL functionality
    fn lsr_immediate(&mut self, value: u8) -> u8 {
        // Bit 7 is set in carry flag
        let first_bit = value & 0b0000_0001;
        if first_bit > 0 {
            self.registers.set_carry()
        } else {
            self.registers.unset_carry()
        }

        let new_value = value >> 1;

        if new_value == 0 {
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        // Not really necessary to check if we need to set_neg() as bit 7 will ALWAYS be 0
        self.registers.unset_neg();

        new_value
    }

    // Opcode: $4A
    // 2 cycles
    fn lsr_accumulator(&mut self) -> u8 {
        let new_accum = self.lsr_immediate(self.registers.accumulator);
        self.registers.accumulator = new_accum;

        2
    }

    // Opcode: $46
    // 5 cycles
    fn lsr_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.lsr_immediate(value);
        self.memory.store_zero_page(addr_lower_byte, value);

        5
    }

    // Opcode: $56
    // 6 cycles
    fn lsr_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.lsr_immediate(value);
        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, value);

        6
    }

    // Opcode: $4E
    // 6 cycles
    fn lsr_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        let value = self.lsr_immediate(value);
        self.memory.store_absolute(address, value);

        6
    }

    // Opcode: $5E
    // 7 cycles
    fn lsr_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.lsr_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_x, value);

        7
    }

    fn lsr_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        let value = self.lsr_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_y, value);
    }

    fn lsr_indirect_y(&mut self, address: u8) {
        let value = self
            .memory
            .fetch_indirect_y(address, self.registers.index_y);
        let value = self.lsr_immediate(value);
        self.memory
            .store_indirect_y(address, self.registers.index_y, value);
    }

    fn lsr_indirect_x(&mut self, address: u8) {
        let value = self
            .memory
            .fetch_indirect_x(address, self.registers.index_x);
        let value = self.lsr_immediate(value);
        self.memory
            .store_indirect_x(address, self.registers.index_x, value);
    }

    /*
     * ROL - Rotate Left
     * Move each of the bits in either A or M one place to the left. Bit 0 is filled with the current value of the carry flag whilst the old bit 7 becomes the new carry flag value.
     */

    fn rol_immediate(&mut self, value: u8) -> u8 {
        // Bit 7 is set in carry flag
        let first_bit = value & 0b1000_0000;

        let new_value = value << 1;
        let new_value = new_value | self.registers.get_carry();

        if first_bit > 0 {
            self.registers.set_carry()
        } else {
            self.registers.unset_carry()
        }

        self.update_zero_negative_flags(new_value);
        new_value
    }

    // Opcode: $2A
    // 2 cycles
    fn rol_accumulator(&mut self) -> u8 {
        self.registers.accumulator = self.rol_immediate(self.registers.accumulator);

        2
    }

    // Opcode: $26
    // 5 cycles
    fn rol_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.rol_immediate(value);
        self.memory.store_zero_page(addr_lower_byte, value);

        5
    }

    // Opcode: $36
    // 6 cycles
    fn rol_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.rol_immediate(value);
        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, value);

        6
    }

    // Opcode: $2E
    // 6 cycles
    fn rol_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        let value = self.rol_immediate(value);
        self.memory.store_absolute(address, value);

        6
    }

    // Opcode: $3E
    // 7 cycles
    fn rol_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.rol_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_x, value);

        7
    }

    fn rol_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        let value = self.rol_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_y, value);
    }

    fn rol_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        let value = self.rol_immediate(value);
        self.memory
            .store_indirect_x(addr_lower_byte, self.registers.index_x, value);
    }

    fn rol_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        let value_after_rol = self.rol_immediate(value);
        self.memory
            .store_indirect_y(addr_lower_byte, self.registers.index_y, value_after_rol);
        value_after_rol
    }

    /*
     * ROR - Rotate Right
     * Move each of the bits in either A or M one place to the right. Bit 7 is filled with the current value of the carry flag whilst the old bit 0 becomes the new carry flag value.
     */

    // Opcode: $6A
    // 2 cycles
    fn ror_immediate(&mut self, value: u8) -> u8 {
        // Bit 7 is set in carry flag
        let last_bit = value & 0b0000_0001;

        let new_value = value >> 1;
        let new_value = new_value | (self.registers.get_carry() << 7);

        if last_bit > 0 {
            self.registers.set_carry()
        } else {
            self.registers.unset_carry()
        }

        self.update_zero_negative_flags(new_value);

        new_value
    }

    // Opcode: $6A
    // 2 cycles
    fn ror_accumulator(&mut self) -> u8 {
        self.registers.accumulator = self.ror_immediate(self.registers.accumulator);

        2
    }

    // Opcode: $66
    // 5 cycles
    fn ror_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.ror_immediate(value);
        self.memory.store_zero_page(addr_lower_byte, value);

        5
    }

    // Opcode: $76
    // 6 cycles
    fn ror_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.ror_immediate(value);
        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, value);

        6
    }

    // Opcode: $6E
    // 6 cycles
    fn ror_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        let value = self.ror_immediate(value);
        self.memory.store_absolute(address, value);

        6
    }

    // Opcode: $7E
    // 7 cycles
    fn ror_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.ror_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_x, value);

        7
    }

    // Helper
    fn ror_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        let value = self.ror_immediate(value);
        self.memory
            .store_absolute_x(address, self.registers.index_y, value);

        7
    }

    // Helper
    fn ror_indirect_x(&mut self, address: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(address, self.registers.index_x);
        let value = self.ror_immediate(value);
        self.memory
            .store_indirect_x(address, self.registers.index_x, value);

        7
    }

    // Helper
    fn ror_indirect_y(&mut self, address: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(address, self.registers.index_y);
        let value = self.ror_immediate(value);
        self.memory
            .store_indirect_y(address, self.registers.index_y, value);

        7
    }

    /*
     * LDA - Load accumulator
     * Loads a byte of memory into the accumulator setting the zero and negative flags as appropriate.
     */

    // Opcode: $A9
    // 2 cycles
    fn lda_immediate(&mut self, value: u8) -> u8 {
        self.registers.accumulator = value;
        self.update_zero_negative_flags(self.registers.accumulator);

        2
    }

    // Opcode: $AD
    // 4 cycles
    fn lda_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.lda_immediate(value);

        4
    }

    // Opcode: $A5
    // 3 cycles
    fn lda_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.lda_immediate(value);

        3
    }

    // Opcode: $B5
    // 4 cycles
    fn lda_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.lda_immediate(value);

        4
    }

    // Opcode: $BD
    // 4 (+1 if page crossed) cycles
    fn lda_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.lda_immediate(value);

        4
    }

    // Opcode: $B9
    // 4 (+1 if page crossed) cycles
    fn lda_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.lda_immediate(value);

        4
    }

    // Opcode: $A1
    // 6 cycles
    fn lda_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.lda_immediate(value);

        6
    }

    // Opcode: $B1
    // 5 (+1 if page crossed) cycles
    fn lda_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.lda_immediate(value);

        5
    }

    /*
     *
     * LDX - Load X Register
     * Loads a byte of memory into the X register setting the zero and negative flags as appropriate.
     *
     */

    // Opcode: $A2
    // 2 cycles
    fn ldx_immediate(&mut self, value: u8) -> u8 {
        self.registers.index_x = value;
        self.update_zero_negative_flags(self.registers.index_x);

        2
    }

    // Opcode: $AE
    // 4 cycles
    fn ldx_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.ldx_immediate(value);

        4
    }

    // Opcode: $BE
    // 4 (+1 if page crossed) cycles
    fn ldx_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.ldx_immediate(value);

        4
    }

    // Opcode: $A6
    // 3 cycles
    fn ldx_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.ldx_immediate(value);

        3
    }

    // Opcode: $B6
    // 4 cycles
    fn ldx_zero_page_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_y);
        self.ldx_immediate(value);

        4
    }

    /*
     *
     * LDY - Load Y Register
     * Loads a byte of memory into the Y register setting the zero and negative flags as appropriate.
     *
     */

    // Opcode: $A0
    // 2 cycles
    fn ldy_immediate(&mut self, value: u8) -> u8 {
        self.registers.index_y = value;
        self.update_zero_negative_flags(self.registers.index_y);

        2
    }

    // Opcode: $AC
    // 4 cycles
    fn ldy_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        self.ldy_immediate(value);

        4
    }

    // Opcode: $BC
    // 4 (+1 if page crossed) cycles
    fn ldy_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.ldy_immediate(value);

        4
    }

    // Opcode: $A4
    // 3 cycles
    fn ldy_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.ldy_immediate(value);

        3
    }

    // Opcode: $B4
    // 4 cycles
    fn ldy_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.ldy_immediate(value);

        4
    }

    /*
     *   TAX - Transfer accumulator to X
     *   Copies the current contents of the accumulator into the X register and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $AA
     *   Cycles: 2
     */
    fn tax(&mut self) -> u8 {
        self.ldx_immediate(self.registers.accumulator);

        2
    }

    /*
     *   TAY - Transfer Accumulator to Y
     *   Copies the current contents of the accumulator into the Y register and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $A8
     *   Cycles: 2
     */
    fn tay(&mut self) -> u8 {
        self.ldy_immediate(self.registers.accumulator);

        2
    }

    /*
     *   TSX - Transfer Stack Pointer to X
     *   Copies the current contents of the stack register into the X register and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $BA
     *   Cycles: 2
     */
    fn tsx(&mut self) -> u8 {
        self.ldx_immediate(self.registers.stack_pointer);

        2
    }

    /*
     *   TXA - Transfer X to accumulator
     *   Copies the current contents of the X register into the accumulator and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $8A
     *   Cycles: 2
     */
    fn txa(&mut self) -> u8 {
        self.lda_immediate(self.registers.index_x);

        2
    }

    /*
     *   TXS - Transfer X to Stack Pointer
     *   Copies the current contents of the X register into the stack register.
     *
     *   Opcode: $9A
     *   Cycles: 2
     */
    fn txs(&mut self) -> u8 {
        self.registers.stack_pointer = self.registers.index_x;

        2
    }

    /*
     *   TYA - Transfer Y to Accumulator
     *   Copies the current contents of the Y register into the accumulator and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $98
     *   Cycles: 2
     */
    fn tya(&mut self) -> u8 {
        self.lda_immediate(self.registers.index_y);

        2
    }

    /*
     *   AND - Logical AND operation
     *   Performs a bit by bit AND operation on the accumulator contents using the contents of a byte of memory.
     */

    // Opcode: $29
    // Cycles: 2
    fn and_immediate(&mut self, value: u8) -> u8 {
        self.registers.accumulator &= value;

        self.update_zero_negative_flags(self.registers.accumulator);

        2
    }

    // Opcode: $25
    // Cycles: 3
    fn and_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.and_immediate(value);

        3
    }

    // Opcode: $35
    // Cycles: 4
    fn and_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);

        self.and_immediate(value);

        4
    }

    // Opcode: $2D
    // Cycles: 4
    fn and_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);

        self.and_immediate(value);

        4
    }

    // Opcode: $3D
    // Cycles: 4 (+1 if page crossed)
    fn and_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.and_immediate(value);

        4
    }

    // Opcode: $39
    // Cycles: 4 (+1 if page crossed)
    fn and_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.and_immediate(value);

        4
    }

    // Opcode: $21
    // Cycles: 6
    fn and_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);

        self.and_immediate(value);

        6
    }

    // Opcode: $31
    // Cycles: 5 (+1 if page crossed)
    fn and_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);

        self.and_immediate(value);

        5
    }

    /*
     *   EOR - Exclusive OR
     *   Perform the XOR operation
     */

    // Opcode: $49
    // Cycles: 2
    fn eor_immediate(&mut self, value: u8) -> u8 {
        self.registers.accumulator ^= value;

        self.update_zero_negative_flags(self.registers.accumulator);

        2
    }

    // Opcode: $45
    // Cycles: 3
    fn eor_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.eor_immediate(value);

        3
    }

    // Opcode: $55
    // Cycles: 4
    fn eor_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);

        self.eor_immediate(value);

        4
    }

    // Opcode: $4D
    // Cycles: 4
    fn eor_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);

        self.eor_immediate(value);

        4
    }

    // Opcode: $5D
    // Cycles: 4 (+1 if page crossed)
    fn eor_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.eor_immediate(value);

        4
    }

    // Opcode: $59
    // Cycles: 4 (+1 if page crossed)
    fn eor_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.eor_immediate(value);

        4
    }

    // Opcode: $41
    // Cycles: 6
    fn eor_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);

        self.eor_immediate(value);

        6
    }

    // Opcode: $51
    // Cycles: 5 (+1 if page crossed)
    fn eor_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);

        self.eor_immediate(value);

        5
    }

    /*
     *   ORA - Logical OR Operation
     *   Perform the logical OR operation
     */

    // Opcode: $09
    // Cycles: 2
    fn ora_immediate(&mut self, value: u8) -> u8 {
        self.registers.accumulator |= value;

        self.update_zero_negative_flags(self.registers.accumulator);

        2
    }

    // Opcode: $05
    // Cycles: 3
    fn ora_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.ora_immediate(value);

        3
    }

    // Opcode: $15
    // Cycles: 4
    fn ora_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);

        self.ora_immediate(value);

        4
    }

    // Opcode: $0D
    // Cycles: 4
    fn ora_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);

        self.ora_immediate(value);

        4
    }

    // Opcode: $1D
    // Cycles: 4 (+1 if page crossed)
    fn ora_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.ora_immediate(value);

        4
    }

    // Opcode: $19
    // Cycles: 4 (+1 if page crossed)
    fn ora_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.ora_immediate(value);

        4
    }

    // Opcode: $01
    // Cycles: 6
    fn ora_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);

        self.ora_immediate(value);

        6
    }

    // Opcode: $11
    // Cycles: 5 (+1 if page crossed)
    fn ora_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);

        self.ora_immediate(value);

        5
    }

    /*
     *   BIT - BIT Test for certain bits
     *   Check if one or more bits are set in target memory location
     */

    // Opcode: $24
    // Cycles: 3
    fn bit_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let result = self.registers.accumulator & value;

        if result == 0 {
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        if value & 0b1000_0000 != 0 {
            self.registers.set_neg();
        } else {
            self.registers.unset_neg();
        }

        if value & 0b0100_0000 != 0 {
            self.registers.set_overflow();
        } else {
            self.registers.unset_overflow();
        }

        3
    }

    // Opcode: $2C
    // Cycles: 4
    fn bit_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address);
        let result = self.registers.accumulator & value;

        if result == 0 {
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        if value & 0b1000_0000 != 0 {
            self.registers.set_neg();
        } else {
            self.registers.unset_neg();
        }

        if value & 0b0100_0000 != 0 {
            self.registers.set_overflow();
        } else {
            self.registers.unset_overflow();
        }

        4
    }

    /*
     *   CLC - Clear Carry Flag
     *   Set the carry flag to zero.
     *
     *   Opcode: $18
     *   Cycles: 2
     */
    fn clc(&mut self) -> u8 {
        self.registers.unset_carry();

        2
    }

    /*
     *   UNUSED FOR NES!
     *   CLD - Clear Decimal Mode
     *   Sets the decimal mode flag to zero.
     *
     *   Opcode: $D8
     *   Cycles: 2
     */
    fn cld(&mut self) -> u8 {
        self.registers.unset_decimal();

        2
    }

    /*
     *   CLI - Clear Interrupt Disable
     *   Clears the interrupt disable flag allowing normal interrupt requests to be serviced.
     *
     *   Opcode: $58
     *   Cycles: 2
     */
    fn cli(&mut self) -> u8 {
        self.registers.unset_interrupt_disable();

        2
    }

    /*
     *   CLV - Clear Overflow Flag
     *   Clears the overflow flag.
     *
     *   Opcode: $B8
     *   Cycles: 2
     */
    fn clv(&mut self) -> u8 {
        self.registers.unset_overflow();

        2
    }

    /*
     *   SEC - Set Carry Flag
     *   Set the carry flag to one.
     *
     *   Opcode: $38
     *   Cycles: 2
     */
    fn sec(&mut self) -> u8 {
        self.registers.set_carry();

        2
    }

    /*
     *    UNUSED FOR NES!
     *
     *   SED - Set Decimal Flag
     *   Set the decimal mode flag to one.
     *
     *   Opcode: $F8
     *   Cycles: 2
     */
    fn sed(&mut self) -> u8 {
        self.registers.set_decimal();
        2
    }

    /*
     *   SEI - Set Interrupt Disable
     *   Set the interrupt disable flag to one.
     *
     *   Opcode: $78
     *   Cycles: 2
     */
    fn sei(&mut self) -> u8 {
        self.registers.set_interrupt_disable();

        2
    }

    /*
     *   PHA - Push accumulator
     *   Pushes a copy of the accumulator on to the stack.
     *
     *   Opcode: $48
     *   Cycles: 3
     */
    fn pha(&mut self) -> u8 {
        self.stack_push(self.registers.accumulator);

        3
    }

    /*
     *   PHP - Push Processor Status
     *   Pushes a copy of the status flags on to the stack.
     *
     *   Opcode: $08
     *   Cycles: 3
     */
    fn php(&mut self) -> u8 {
        self.stack_push(self.registers.processor_status | 0x10);

        3
    }

    /*
     *   PLA - Pull Accumulator
     *   Pulls an 8 bit value from the stack and into the accumulator. The zero and negative flags are set as appropriate.
     *
     *   Opcode: $68
     *   Cycles: 4
     */
    fn pla(&mut self) -> u8 {
        let val = self.stack_pop();
        self.lda_immediate(val);

        4
    }

    /*
     *   PLP - Pull Processor Status
     *   Pulls an 8 bit value from the stack and into the processor flags. The flags will take on new states as determined by the value pulled.
     *
     *   Opcode: $28
     *   Cycles: 4
     */
    fn plp(&mut self) -> u8 {
        let val = self.stack_pop();
        self.registers.processor_status = val | 0b100000;
        self.registers.unset_break();

        4
    }

    /*
     *   JMP - Jump
     *   Sets the program counter to the address specified by the operand.
     *   For compatibility always ensure the indirect vector is not at the end of the page.
     */

    // Opcode: $4C
    // Cycles: 3
    fn jmp_absolute(&mut self, address: u16) -> u8 {
        self.registers.program_counter = address;

        3
    }

    // Opcode: $6C
    // Cycles: 5
    fn jmp_indirect(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_indirect_quirk(address);
        self.registers.program_counter = value;

        5
    }

    /*
     *   JSR - Jump to Subroutine
     *   The JSR instruction pushes the address (minus one) of the return point on to the stack and then sets the program counter to the target memory address.
     *
     *   Opcode: $20
     *   Cycles: 6
     */
    fn jsr(&mut self, address: u16) -> u8 {
        let pc_high = ((self.registers.program_counter + 2) >> 8) as u8;
        let pc_low = ((self.registers.program_counter + 2) & 0xFF) as u8;
        self.stack_push(pc_high);
        self.stack_push(pc_low);

        self.registers.program_counter = address;

        6
    }

    /*
     *   BCC - Branch if Carry Clear
     *   If the carry flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $90
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bcc(&mut self, offset: u8) -> u8 {
        if self.registers.get_carry() == 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);
            3
        } else {
            2
        }
    }

    /*
     *   BCS - Branch if Carry Set
     *   If the carry flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $B0
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bcs(&mut self, offset: u8) -> u8 {
        if self.registers.get_carry() > 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);
            3
        } else {
            2
        }
    }

    /*
     *   BEQ - Branch if Equal
     *   If the zero flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $F0
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn beq(&mut self, offset: u8) -> u8 {
        if self.registers.get_zero() > 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);

            3
        } else {
            2
        }
    }

    /*
     *   BMI - Branch if Minus
     *   If the negative flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $30
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bmi(&mut self, offset: u8) -> u8 {
        if self.registers.get_neg() > 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);
            3
        } else {
            2
        }
    }

    /*
     *   BNE - Branch if Not Equal
     *   If the zero flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $D0
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bne(&mut self, offset: u8) -> u8 {
        if self.registers.get_zero() == 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);
            3
        } else {
            2
        }
    }

    /*
     *   BPL - Branch if Positive
     *   If the negative flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $10
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bpl(&mut self, offset: u8) -> u8 {
        if self.registers.get_neg() == 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);
            3
        } else {
            2
        }
    }

    /*
     *   BVC - Branch if Overflow Clear
     *   If the overflow flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $50
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bvc(&mut self, offset: u8) -> u8 {
        if self.registers.get_overflow() == 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);
            3
        } else {
            2
        }
    }

    /*
     *   BVS - Branch if Overflow Set
     *   If the overflow flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $70
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bvs(&mut self, offset: u8) -> u8 {
        if self.registers.get_overflow() > 0 {
            self.registers.program_counter = self
                .registers
                .program_counter
                .wrapping_add_signed(offset as i8 as i16);
            3
        } else {
            2
        }
    }

    /*
     *   STA - Store Accumulator
     *   Store the value of the accumulator into memory
     */

    // Opcode: $85
    // Cycles: 3
    fn sta_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory
            .store_zero_page(addr_lower_byte, self.registers.accumulator);

        3
    }

    // Opcode: $95
    // Cycles: 4
    fn sta_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory.store_zero_page_x(
            addr_lower_byte,
            self.registers.index_x,
            self.registers.accumulator,
        );

        4
    }

    // Opcode: $8D
    // Cycles: 4
    fn sta_absolute(&mut self, address: u16) -> u8 {
        self.memory
            .store_absolute(address, self.registers.accumulator);

        4
    }

    // Opcode: $9D
    // Cycles: 5
    fn sta_absolute_x(&mut self, address: u16) -> u8 {
        self.memory
            .store_absolute_x(address, self.registers.index_x, self.registers.accumulator);

        5
    }

    // Opcode: $99
    // Cycles: 5
    fn sta_absolute_y(&mut self, address: u16) -> u8 {
        self.memory
            .store_absolute_x(address, self.registers.index_y, self.registers.accumulator);

        5
    }

    // Opcode: $81
    // Cycles: 6
    fn sta_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory.store_indirect_x(
            addr_lower_byte,
            self.registers.index_x,
            self.registers.accumulator,
        );

        6
    }

    // Opcode: $91
    // Cycles: 6
    fn sta_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory.store_indirect_y(
            addr_lower_byte,
            self.registers.index_y,
            self.registers.accumulator,
        );

        6
    }

    /*
     *   STX - Store the value at the X register
     *   Store the value of the X register into memory
     */

    // Opcode: $86
    // Cycles: 3
    fn stx_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory
            .store_zero_page(addr_lower_byte, self.registers.index_x);

        3
    }

    // Opcode: $96
    // Cycles: 4
    fn stx_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory.store_zero_page_x(
            addr_lower_byte,
            self.registers.index_y,
            self.registers.index_x,
        );

        4
    }

    // Opcode: $8E
    // Cycles: 4
    fn stx_absolute(&mut self, address: u16) -> u8 {
        self.memory.store_absolute(address, self.registers.index_x);

        4
    }

    /*
     *   STY - Store the value at the Y register
     *   Store the value of the Y register into memory
     */

    // Opcode: $84
    // Cycles: 3
    fn sty_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory
            .store_zero_page(addr_lower_byte, self.registers.index_y);

        3
    }

    // Opcode: $94
    // Cycles: 4
    fn sty_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.memory.store_zero_page_x(
            addr_lower_byte,
            self.registers.index_x,
            self.registers.index_y,
        );

        4
    }

    // Opcode: $8C
    // Cycles: 4
    fn sty_absolute(&mut self, address: u16) -> u8 {
        self.memory.store_absolute(address, self.registers.index_y);

        4
    }

    /*
     *   INC - Increment Memory
     *   Increment the value at a specified memory location
     */

    // Opcode: $E6
    // Cycles: 5
    fn inc_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let new_val = self.memory.fetch_zero_page(addr_lower_byte).wrapping_add(1);

        self.memory.store_zero_page(addr_lower_byte, new_val);
        self.update_zero_negative_flags(new_val);

        5
    }

    // Opcode: $F6
    // Cycles: 6
    fn inc_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let new_val = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x)
            .wrapping_add(1);

        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, new_val);
        self.update_zero_negative_flags(new_val);

        6
    }

    // Opcode: $EE
    // Cycles: 6
    fn inc_absolute(&mut self, address: u16) -> u8 {
        let new_val = self.memory.fetch_absolute(address).wrapping_add(1);

        self.memory.store_absolute(address, new_val);
        self.update_zero_negative_flags(new_val);

        6
    }

    // Opcode: $FE
    // Cycles: 7
    fn inc_absolute_x(&mut self, address: u16) -> u8 {
        let new_val = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x)
            .wrapping_add(1);

        self.memory
            .store_absolute_x(address, self.registers.index_x, new_val);
        self.update_zero_negative_flags(new_val);

        7
    }

    /*
     *   INX - Increment X Register
     *   Increment the value at the X Register
     *
     *   Opcode: $E8
     *   Cycles: 2
     */

    fn inx_implied(&mut self) -> u8 {
        self.registers.index_x = self.registers.index_x.wrapping_add(1);
        self.update_zero_negative_flags(self.registers.index_x);

        2
    }

    /*
     *   INY - Increment Y Register
     *   Increment the value at the Y Register
     *
     *   Opcode: $C8
     *   Cycles: 2
     */

    fn iny_implied(&mut self) -> u8 {
        self.registers.index_y = self.registers.index_y.wrapping_add(1);
        self.update_zero_negative_flags(self.registers.index_y);

        2
    }

    /*
     *   DEC - Decrement Memory
     *   Decrement the value at a specified memory location
     */

    // Opcode: $C6
    // Cycles: 5
    fn dec_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let new_val = self.memory.fetch_zero_page(addr_lower_byte).wrapping_sub(1);

        self.memory.store_zero_page(addr_lower_byte, new_val);
        self.update_zero_negative_flags(new_val);

        5
    }

    // Opcode: $D6
    // Cycles: 6
    fn dnc_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let new_val = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x)
            .wrapping_sub(1);

        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, new_val);
        self.update_zero_negative_flags(new_val);

        6
    }

    // Opcode: $CE
    // Cycles: 6
    fn dec_absolute(&mut self, address: u16) -> u8 {
        let new_val = self.memory.fetch_absolute(address).wrapping_sub(1);

        self.memory.store_absolute(address, new_val);
        self.update_zero_negative_flags(new_val);

        6
    }

    // Opcode: $DE
    // Cycles: 7
    fn dec_absolute_x(&mut self, address: u16) -> u8 {
        let new_val = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x)
            .wrapping_sub(1);

        self.memory
            .store_absolute_x(address, self.registers.index_x, new_val);
        self.update_zero_negative_flags(new_val);

        7
    }

    /*
     *   DEX - Decrement X Register
     *   Decrement the value at the X Register
     *
     *   Opcode: $CA
     *   Cycles: 2
     */

    fn dex_implied(&mut self) -> u8 {
        self.registers.index_x = self.registers.index_x.wrapping_sub(1);
        self.update_zero_negative_flags(self.registers.index_x);

        2
    }

    /*
     *   DEY - Decrement Y Register
     *   Decrement the value at the Y Register
     *
     *   Opcode: $88
     *   Cycles: 2
     */

    fn dey_implied(&mut self) -> u8 {
        self.registers.index_y = self.registers.index_y.wrapping_sub(1);
        self.update_zero_negative_flags(self.registers.index_y);

        2
    }

    /*
     *   BRK - Force Interrupt
     *   Forces the generation of an interrupt request
     *
     *   Opcode: $00
     *   Cycles: 7
     */

    pub fn brk_implied(&mut self) -> u8 {
        let pc_high = ((self.registers.program_counter + 2) >> 8) as u8;
        self.stack_push(pc_high);

        // Push low byte
        let pc_low = ((self.registers.program_counter + 2) & 0xFF) as u8;
        self.stack_push(pc_low);

        self.stack_push(self.registers.processor_status | 0x10);

        let irq_vector_low = self.memory.fetch_absolute(0xFFFE) as u16;
        let irq_vector_high = self.memory.fetch_absolute(0xFFFF) as u16;
        let irq_vector = irq_vector_low | (irq_vector_high << 8);
        self.registers.program_counter = irq_vector;

        // self.registers.set_break();
        self.registers.set_interrupt_disable();

        7
    }

    pub fn generate_nmi(&mut self) -> u8 {
        let pc_high = ((self.registers.program_counter) >> 8) as u8;
        self.stack_push(pc_high);

        let pc_low = ((self.registers.program_counter) & 0xFF) as u8;
        self.stack_push(pc_low);

        self.stack_push(self.registers.processor_status);

        let nmi_vector_low = self.memory.fetch_absolute(0xFFFA) as u16;
        let nmi_vector_high = self.memory.fetch_absolute(0xFFFB) as u16;
        let nmi_vector = nmi_vector_low | (nmi_vector_high << 8);
        self.registers.program_counter = nmi_vector;

        // self.registers.set_break();
        self.registers.set_interrupt_disable();

        7
    }

    /*
     *   NOP - No Operation
     *   Simply increments the PC to the next instruction
     *
     *   Opcode: $EA
     *   Cycles: 2
     */

    fn nop_implied(&mut self) -> u8 {
        2
    }

    /*
     *   RTI - Return from Interrupt
     *   Used at the end of an interrupt processing routine
     *
     *   Opcode: $40
     *   Cycles: 6
     */

    fn rti_implied(&mut self) -> u8 {
        let status = self.stack_pop();
        self.registers.processor_status = status | 0b100000;

        let pc_low = self.stack_pop() as u16;
        let pc_high = self.stack_pop() as u16;

        let pc = (pc_high << 8) | pc_low;
        self.registers.program_counter = pc;

        self.registers.unset_break();

        6
    }

    /*
     *   RTS - Return from Subroutine
     *   The RTS instruction is used at the end of a subroutine to return to the calling routine. It pulls the program counter (minus one) from the stack.
     *
     *   Opcode: $60
     *   Cycles: 6
     */

    fn rts(&mut self) -> u8 {
        let pc_low = self.stack_pop() as u16;
        let pc_high = self.stack_pop() as u16;

        let pc = (pc_high << 8) | pc_low;

        self.registers.program_counter = pc;

        6
    }

    // ILLEGAL OPCODES

    /*
     *   USBC: SBC oper + NOP
     *   effectively same as normal SBC immediate, instr. E9.
     *   A - M - C -> A
     *
     *   Opcode: $EB
     *   Cycles: 2
     */
    fn usbc(&mut self, val: u8) -> u8 {
        self.sbc_immediate(val);
        2
    }

    /*
     *   RLA: ROL oper + AND oper
     *   M = C <- [76543210] <- C, A AND M -> A
     */

    // Opcode: $27
    // Cycles: 5
    fn rla_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.rol_zero_page(addr_lower_byte);
        self.and_zero_page(addr_lower_byte);

        5
    }

    // Opcode: $37
    // Cycles: 6
    fn rla_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.rol_zero_page_x(addr_lower_byte);
        self.and_zero_page_x(addr_lower_byte);

        6
    }

    // Opcode: $2F
    // Cycles: 6
    fn rla_absolute(&mut self, address: u16) -> u8 {
        self.rol_absolute(address);
        self.and_absolute(address);

        6
    }

    // Opcode: $3F
    // Cycles: 7
    fn rla_absolute_x(&mut self, address: u16) -> u8 {
        self.rol_absolute_x(address);
        self.and_absolute_x(address);

        7
    }

    // Opcode: $3B
    // Cycles: 7
    fn rla_absolute_y(&mut self, address: u16) -> u8 {
        self.rol_absolute_y(address);
        self.and_absolute_y(address);

        7
    }

    // Opcode: $23
    // Cycles: 8
    fn rla_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.rol_indirect_x(addr_lower_byte);
        self.and_indirect_x(addr_lower_byte);

        8
    }

    // Opcode: $33
    // Cycles: 8
    fn rla_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.rol_indirect_y(addr_lower_byte);
        // TODO: (BUG) In some cases, you cannot read the same address after store_indirect_{x,y} if the addr_lower_byte is modified itself
        self.and_immediate(value);

        8
    }

    /*
     *   RRA: ROR oper + ADC oper
     *   M = C -> [76543210] -> C, A + M + C -> A, C
     */

    // Opcode: $67
    // Cycles: 5
    fn rra_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.ror_zero_page(addr_lower_byte);
        self.adc_zero_page(addr_lower_byte);

        5
    }

    // Opcode: $77
    // Cycles: 6
    fn rra_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.ror_zero_page_x(addr_lower_byte);
        self.adc_zero_page_x(addr_lower_byte);

        6
    }

    // Opcode: $6F
    // Cycles: 6
    fn rra_absolute(&mut self, address: u16) -> u8 {
        self.ror_absolute(address);
        self.adc_absolute(address);

        6
    }

    // Opcode: $7F
    // Cycles: 7
    fn rra_absolute_x(&mut self, address: u16) -> u8 {
        self.ror_absolute_x(address);
        self.adc_absolute_x(address);

        7
    }

    // Opcode: $7B
    // Cycles: 7
    fn rra_absolute_y(&mut self, address: u16) -> u8 {
        self.ror_absolute_y(address);
        self.adc_absolute_y(address);

        7
    }

    // Opcode: $63
    // Cycles: 8
    fn rra_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.ror_indirect_x(addr_lower_byte);
        self.adc_indirect_x(addr_lower_byte);

        8
    }

    // Opcode: $73
    // Cycles: 8
    fn rra_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        self.ror_indirect_y(addr_lower_byte);
        self.adc_indirect_y(addr_lower_byte);

        8
    }

    /*
     *   SLO: ASL oper + ORA oper
     *   M = C <- [76543210] <- 0, A OR M -> A
     */

    // Opcode: $07
    // Cycles: 5
    fn slo_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.asl_zero_page(addr_lower_byte);
        self.ora_zero_page(addr_lower_byte);

        5
    }

    // Opcode: $17
    // Cycles: 6
    fn slo_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.asl_zero_page_x(addr_lower_byte);
        self.ora_zero_page_x(addr_lower_byte);

        6
    }

    // Opcode: $0F
    // Cycles: 6
    fn slo_absolute(&mut self, address: u16) -> u8 {
        self.asl_absolute(address);
        self.ora_absolute(address);

        6
    }

    // Opcode: $1F
    // Cycles: 7
    fn slo_absolute_x(&mut self, address: u16) -> u8 {
        self.asl_absolute_x(address);
        self.ora_absolute_x(address);

        7
    }

    // Opcode: $1B
    // Cycles: 7
    fn slo_absolute_y(&mut self, address: u16) -> u8 {
        self.asl_absolute_y(address);
        self.ora_absolute_y(address);

        7
    }

    // Opcode: $03
    // Cycles: 8
    fn slo_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.asl_indirect_x(addr_lower_byte);
        self.ora_indirect_x(addr_lower_byte);

        8
    }

    // Opcode: $13
    // Cycles: 8
    fn slo_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        self.asl_indirect_y(addr_lower_byte);
        self.ora_indirect_y(addr_lower_byte);

        8
    }

    /*
     *   SRE: LSR oper + EOR oper
     *   M = 0 -> [76543210] -> C, A EOR M -> A
     */

    // Opcode: $47
    // Cycles: 5
    fn sre_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.lsr_zero_page(addr_lower_byte);
        self.eor_zero_page(addr_lower_byte);

        5
    }

    // Opcode: $57
    // Cycles: 6
    fn sre_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.lsr_zero_page_x(addr_lower_byte);
        self.eor_zero_page_x(addr_lower_byte);

        6
    }

    // Opcode: $4F
    // Cycles: 6
    fn sre_absolute(&mut self, address: u16) -> u8 {
        self.lsr_absolute(address);
        self.eor_absolute(address);

        6
    }

    // Opcode: $5F
    // Cycles: 7
    fn sre_absolute_x(&mut self, address: u16) -> u8 {
        self.lsr_absolute_x(address);
        self.eor_absolute_x(address);

        7
    }

    // Opcode: $5B
    // Cycles: 7
    fn sre_absolute_y(&mut self, address: u16) -> u8 {
        self.lsr_absolute_y(address);
        self.eor_absolute_y(address);

        7
    }

    // Opcode: $43
    // Cycles: 8
    fn sre_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.lsr_indirect_x(addr_lower_byte);
        self.eor_indirect_x(addr_lower_byte);

        8
    }

    // Opcode: $53
    // Cycles: 8
    fn sre_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        self.lsr_indirect_y(addr_lower_byte);
        self.eor_indirect_y(addr_lower_byte);

        8
    }

    /*
     *   SAX: A and X are put on the bus at the same time (resulting effectively in an AND operation) and stored in M
     */

    // Opcode: $83
    // Cycles: 6
    fn sax_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.registers.index_x & self.registers.accumulator;
        self.memory
            .store_indirect_x(addr_lower_byte, self.registers.index_x, value);

        6
    }

    // Opcode: $87
    // Cycles: 3
    fn sax_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.registers.index_x & self.registers.accumulator;
        self.memory.store_zero_page(addr_lower_byte, value);

        3
    }

    // Opcode: $8F
    // Cycles: 4
    fn sax_absolute(&mut self, address: u16) -> u8 {
        let value = self.registers.index_x & self.registers.accumulator;
        self.memory.store_absolute(address, value);

        4
    }

    // Opcode: $97
    // Cycles: 4
    fn sax_zero_page_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.registers.index_x & self.registers.accumulator;
        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_y, value);
        4
    }

    /*
     *  DCP: DEC oper + CMP oper
     */

    // Opcde: $C3
    // Cycles: 8
    fn dcp_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x)
            .wrapping_sub(1);
        self.memory
            .store_indirect_x(addr_lower_byte, self.registers.index_x, value);

        self.cmp_indirect_x(addr_lower_byte);

        8
    }

    // Opcde: $D3
    // Cycles: 8
    fn dcp_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y)
            .wrapping_sub(1);
        self.memory
            .store_indirect_y(addr_lower_byte, self.registers.index_y, value);

        self.cmp_indirect_y(addr_lower_byte);

        8
    }

    // Opcde: $C7
    // Cycles: 5
    fn dcp_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte).wrapping_sub(1);
        self.memory.store_zero_page(addr_lower_byte, value);

        self.cmp_zero_page(addr_lower_byte);

        5
    }

    // Opcde: $D7
    // Cycles: 6
    fn dcp_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x)
            .wrapping_sub(1);
        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, value);

        self.cmp_zero_page_x(addr_lower_byte);

        6
    }

    // Opcde: $CF
    // Cycles: 6
    fn dcp_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address).wrapping_sub(1);
        self.memory.store_absolute(address, value);

        self.cmp_absolute(address);

        6
    }

    // Opcde: $DF
    // Cycles: 7
    fn dcp_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x)
            .wrapping_sub(1);
        self.memory
            .store_absolute_x(address, self.registers.index_x, value);

        self.cmp_absolute_x(address);

        7
    }

    // Opcde: $DB
    // Cycles: 7
    fn dcp_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y)
            .wrapping_sub(1);
        self.memory
            .store_absolute_x(address, self.registers.index_y, value);

        self.cmp_absolute_y(address);

        7
    }

    /*
     *  INC oper + SBC oper
     */

    // Opcode: $E3
    // Cycles: 8
    fn isb_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x)
            .wrapping_add(1);

        // TODO: (BUG) In some cases, you cannot read the same address after store_indirect_{x,y} if the addr_lower_byte is modified itself
        self.memory
            .store_indirect_x(addr_lower_byte, self.registers.index_x, value);

        self.sbc_immediate(value);

        8
    }

    // Opcode: $F3
    // Cycles: 8
    fn isb_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y)
            .wrapping_add(1);
        self.memory
            .store_indirect_y(addr_lower_byte, self.registers.index_y, value);

        self.sbc_indirect_y(addr_lower_byte);

        8
    }

    // Opcode: $E7
    // Cycles: 5
    fn isb_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self.memory.fetch_zero_page(addr_lower_byte).wrapping_add(1);
        self.memory.store_zero_page(addr_lower_byte, value);

        self.sbc_zero_page(addr_lower_byte);

        5
    }

    // Opcode: $F7
    // Cycles: 6
    fn isb_zero_page_x(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x)
            .wrapping_add(1);
        self.memory
            .store_zero_page_x(addr_lower_byte, self.registers.index_x, value);

        self.sbc_zero_page_x(addr_lower_byte);

        6
    }

    // Opcode: $EF
    // Cycles: 6
    fn isb_absolute(&mut self, address: u16) -> u8 {
        let value = self.memory.fetch_absolute(address).wrapping_add(1);
        self.memory.store_absolute(address, value);

        self.sbc_absolute(address);

        6
    }

    // Opcode: $FB
    // Cycles: 7
    fn isb_absolute_y(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y)
            .wrapping_add(1);
        self.memory
            .store_absolute_x(address, self.registers.index_y, value);

        self.sbc_absolute_y(address);

        7
    }

    // Opcode: $FF
    // Cycles: 7
    fn isb_absolute_x(&mut self, address: u16) -> u8 {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x)
            .wrapping_add(1);
        self.memory
            .store_absolute_x(address, self.registers.index_x, value);

        self.sbc_absolute_x(address);

        7
    }

    /*
     *  LDA oper + LDX oper
     */

    // Opcode: $A3
    // Cycles: 6
    fn lax_indirect_x(&mut self, addr_lower_byte: u8) -> u8 {
        self.lda_indirect_x(addr_lower_byte);
        let val = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.ldx_immediate(val);
        6
    }

    // Opcode: $B3
    // Cycles: 5 (+1 if page boundary is crossed)
    fn lax_indirect_y(&mut self, addr_lower_byte: u8) -> u8 {
        self.lda_indirect_y(addr_lower_byte);
        let val = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.ldx_immediate(val);
        5
    }

    // Opcode: $A7
    // Cycles: 3
    fn lax_zero_page(&mut self, addr_lower_byte: u8) -> u8 {
        self.lda_zero_page(addr_lower_byte);
        self.ldx_zero_page(addr_lower_byte);
        3
    }

    // Opcode: $B7
    // Cycles: 4
    fn lax_zero_page_y(&mut self, addr_lower_byte: u8) -> u8 {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_y);
        self.lda_immediate(value);
        self.ldx_zero_page_y(addr_lower_byte);
        4
    }

    // Opcode: $AF
    // Cycles: 4
    fn lax_absolute(&mut self, address: u16) -> u8 {
        self.lda_absolute(address);
        self.ldx_absolute(address);
        4
    }

    // Opcode: $BF
    // Cycles: 4 (+1 if page boundary crossed)
    fn lax_absolute_y(&mut self, address: u16) -> u8 {
        self.lda_absolute_y(address);
        self.ldx_absolute_y(address);
        4
    }

    fn fetch_u16(&mut self, addr: u16) -> u16 {
        (self.memory.fetch_absolute(addr) as u16)
            + (self.memory.fetch_absolute(addr.wrapping_add(1)) as u16 * 256)
    }

    /*
     * Maps opcodes to methods and is responsible for decoding & executing
     */
    fn decode_execute(&mut self, opcode: u8) -> (u8, u8) {
        macro_rules! handle_opcode_zerobyte {
            ($self:ident, $method:ident) => {{
                ($self.$method(), 0)
            }};
        }

        macro_rules! handle_opcode_onebyte {
            ($self:ident, $method:ident) => {{
                ($self.$method(), 1)
            }};
        }

        macro_rules! handle_opcode_twobytes {
            ($self:ident, $method:ident) => {{
                let value = $self
                    .memory
                    .fetch_absolute($self.registers.program_counter.wrapping_add(1));
                ($self.$method(value), 2)
            }};
        }

        macro_rules! handle_opcode_threebytes {
            ($self:ident, $method:ident) => {{
                let value = self.fetch_u16($self.registers.program_counter.wrapping_add(1));
                ($self.$method(value), 3)
            }};
        }

        macro_rules! handle_opcode_jump {
            ($self:ident, $method:ident) => {{
                let value = self.fetch_u16($self.registers.program_counter.wrapping_add(1));
                ($self.$method(value.into()), 0)
            }};
        }

        match opcode {
            0x00 => handle_opcode_zerobyte!(self, brk_implied),
            0x01 => handle_opcode_twobytes!(self, ora_indirect_x),
            0x05 => handle_opcode_twobytes!(self, ora_zero_page),
            0x06 => handle_opcode_twobytes!(self, asl_zero_page),
            0x08 => handle_opcode_onebyte!(self, php),
            0x09 => handle_opcode_twobytes!(self, ora_immediate),
            0x0A => handle_opcode_onebyte!(self, asl_accumulator),
            0x0D => handle_opcode_threebytes!(self, ora_absolute),
            0x0E => handle_opcode_threebytes!(self, asl_absolute),
            0x10 => handle_opcode_twobytes!(self, bpl),
            0x11 => handle_opcode_twobytes!(self, ora_indirect_y),
            0x15 => handle_opcode_twobytes!(self, ora_zero_page_x),
            0x16 => handle_opcode_twobytes!(self, asl_zero_page_x),
            0x18 => handle_opcode_onebyte!(self, clc),
            0x19 => handle_opcode_threebytes!(self, ora_absolute_y),
            0x1D => handle_opcode_threebytes!(self, ora_absolute_x),
            0x1E => handle_opcode_threebytes!(self, asl_absolute_x),
            0x20 => handle_opcode_jump!(self, jsr),
            0x21 => handle_opcode_twobytes!(self, and_indirect_x),
            0x24 => handle_opcode_twobytes!(self, bit_zero_page),
            0x25 => handle_opcode_twobytes!(self, and_zero_page),
            0x26 => handle_opcode_twobytes!(self, rol_zero_page),
            0x28 => handle_opcode_onebyte!(self, plp),
            0x29 => handle_opcode_twobytes!(self, and_immediate),
            0x2A => handle_opcode_onebyte!(self, rol_accumulator),
            0x2C => handle_opcode_threebytes!(self, bit_absolute),
            0x2D => handle_opcode_threebytes!(self, and_absolute),
            0x2E => handle_opcode_threebytes!(self, rol_absolute),
            0x30 => handle_opcode_twobytes!(self, bmi),
            0x31 => handle_opcode_twobytes!(self, and_indirect_y),
            0x35 => handle_opcode_twobytes!(self, and_zero_page_x),
            0x36 => handle_opcode_twobytes!(self, rol_zero_page_x),
            0x38 => handle_opcode_onebyte!(self, sec),
            0x39 => handle_opcode_threebytes!(self, and_absolute_y),
            0x3D => handle_opcode_threebytes!(self, and_absolute_x),
            0x3E => handle_opcode_threebytes!(self, rol_absolute_x),
            0x40 => handle_opcode_zerobyte!(self, rti_implied),
            0x41 => handle_opcode_twobytes!(self, eor_indirect_x),
            0x45 => handle_opcode_twobytes!(self, eor_zero_page),
            0x46 => handle_opcode_twobytes!(self, lsr_zero_page),
            0x48 => handle_opcode_onebyte!(self, pha),
            0x49 => handle_opcode_twobytes!(self, eor_immediate),
            0x4A => handle_opcode_onebyte!(self, lsr_accumulator),
            0x4C => handle_opcode_jump!(self, jmp_absolute),
            0x4D => handle_opcode_threebytes!(self, eor_absolute),
            0x4E => handle_opcode_threebytes!(self, lsr_absolute),
            0x50 => handle_opcode_twobytes!(self, bvc),
            0x51 => handle_opcode_twobytes!(self, eor_indirect_y),
            0x55 => handle_opcode_twobytes!(self, eor_zero_page_x),
            0x56 => handle_opcode_twobytes!(self, lsr_zero_page_x),
            0x58 => handle_opcode_onebyte!(self, cli),
            0x59 => handle_opcode_threebytes!(self, eor_absolute_y),
            0x5D => handle_opcode_threebytes!(self, eor_absolute_x),
            0x5E => handle_opcode_threebytes!(self, lsr_absolute_x),
            0x60 => handle_opcode_onebyte!(self, rts),
            0x61 => handle_opcode_twobytes!(self, adc_indirect_x),
            0x65 => handle_opcode_twobytes!(self, adc_zero_page),
            0x66 => handle_opcode_twobytes!(self, ror_zero_page),
            0x68 => handle_opcode_onebyte!(self, pla),
            0x69 => handle_opcode_twobytes!(self, adc_immediate),
            0x6A => handle_opcode_onebyte!(self, ror_accumulator),
            0x6C => handle_opcode_jump!(self, jmp_indirect),
            0x6D => handle_opcode_threebytes!(self, adc_absolute),
            0x6E => handle_opcode_threebytes!(self, ror_absolute),
            0x70 => handle_opcode_twobytes!(self, bvs),
            0x71 => handle_opcode_twobytes!(self, adc_indirect_y),
            0x75 => handle_opcode_twobytes!(self, adc_zero_page_x),
            0x76 => handle_opcode_twobytes!(self, ror_zero_page_x),
            0x78 => handle_opcode_onebyte!(self, sei),
            0x79 => handle_opcode_threebytes!(self, adc_absolute_y),
            0x7D => handle_opcode_threebytes!(self, adc_absolute_x),
            0x7E => handle_opcode_threebytes!(self, ror_absolute_x),
            0x81 => handle_opcode_twobytes!(self, sta_indirect_x),
            0x84 => handle_opcode_twobytes!(self, sty_zero_page),
            0x85 => handle_opcode_twobytes!(self, sta_zero_page),
            0x86 => handle_opcode_twobytes!(self, stx_zero_page),
            0x88 => handle_opcode_onebyte!(self, dey_implied),
            0x8A => handle_opcode_onebyte!(self, txa),
            0x8C => handle_opcode_threebytes!(self, sty_absolute),
            0x8D => handle_opcode_threebytes!(self, sta_absolute),
            0x8E => handle_opcode_threebytes!(self, stx_absolute),
            0x90 => handle_opcode_twobytes!(self, bcc),
            0x91 => handle_opcode_twobytes!(self, sta_indirect_y),
            0x94 => handle_opcode_twobytes!(self, sty_zero_page_x),
            0x95 => handle_opcode_twobytes!(self, sta_zero_page_x),
            0x96 => handle_opcode_twobytes!(self, stx_zero_page_x),
            0x98 => handle_opcode_onebyte!(self, tya),
            0x99 => handle_opcode_threebytes!(self, sta_absolute_y),
            0x9A => handle_opcode_onebyte!(self, txs),
            0x9D => handle_opcode_threebytes!(self, sta_absolute_x),
            0xA0 => handle_opcode_twobytes!(self, ldy_immediate),
            0xA1 => handle_opcode_twobytes!(self, lda_indirect_x),
            0xA2 => handle_opcode_twobytes!(self, ldx_immediate),
            0xA4 => handle_opcode_twobytes!(self, ldy_zero_page),
            0xA5 => handle_opcode_twobytes!(self, lda_zero_page),
            0xA6 => handle_opcode_twobytes!(self, ldx_zero_page),
            0xA8 => handle_opcode_onebyte!(self, tay),
            0xA9 => handle_opcode_twobytes!(self, lda_immediate),
            0xAA => handle_opcode_onebyte!(self, tax),
            0xAC => handle_opcode_threebytes!(self, ldy_absolute),
            0xAD => handle_opcode_threebytes!(self, lda_absolute),
            0xAE => handle_opcode_threebytes!(self, ldx_absolute),
            0xB0 => handle_opcode_twobytes!(self, bcs),
            0xB1 => handle_opcode_twobytes!(self, lda_indirect_y),
            0xB4 => handle_opcode_twobytes!(self, ldy_zero_page_x),
            0xB5 => handle_opcode_twobytes!(self, lda_zero_page_x),
            0xB6 => handle_opcode_twobytes!(self, ldx_zero_page_y),
            0xB8 => handle_opcode_onebyte!(self, clv),
            0xB9 => handle_opcode_threebytes!(self, lda_absolute_y),
            0xBA => handle_opcode_onebyte!(self, tsx),
            0xBC => handle_opcode_threebytes!(self, ldy_absolute_x),
            0xBD => handle_opcode_threebytes!(self, lda_absolute_x),
            0xBE => handle_opcode_threebytes!(self, ldx_absolute_y),
            0xC0 => handle_opcode_twobytes!(self, cpy_immediate),
            0xC1 => handle_opcode_twobytes!(self, cmp_indirect_x),
            0xC4 => handle_opcode_twobytes!(self, cpy_zero_page),
            0xC5 => handle_opcode_twobytes!(self, cmp_zero_page),
            0xC6 => handle_opcode_twobytes!(self, dec_zero_page),
            0xC8 => handle_opcode_onebyte!(self, iny_implied),
            0xC9 => handle_opcode_twobytes!(self, cmp_immediate),
            0xCA => handle_opcode_onebyte!(self, dex_implied),
            0xCC => handle_opcode_threebytes!(self, cpy_absolute),
            0xCD => handle_opcode_threebytes!(self, cmp_absolute),
            0xCE => handle_opcode_threebytes!(self, dec_absolute),
            0xD0 => handle_opcode_twobytes!(self, bne),
            0xD1 => handle_opcode_twobytes!(self, cmp_indirect_y),
            0xD5 => handle_opcode_twobytes!(self, cmp_zero_page_x),
            0xD6 => handle_opcode_twobytes!(self, dnc_zero_page_x),
            0xD8 => handle_opcode_onebyte!(self, cld),
            0xD9 => handle_opcode_threebytes!(self, cmp_absolute_y),
            0xDD => handle_opcode_threebytes!(self, cmp_absolute_x),
            0xDE => handle_opcode_threebytes!(self, dec_absolute_x),
            0xE0 => handle_opcode_twobytes!(self, cpx_immediate),
            0xE1 => handle_opcode_twobytes!(self, sbc_indirect_x),
            0xE4 => handle_opcode_twobytes!(self, cpx_zero_page),
            0xE5 => handle_opcode_twobytes!(self, sbc_zero_page),
            0xE6 => handle_opcode_twobytes!(self, inc_zero_page),
            0xE8 => handle_opcode_onebyte!(self, inx_implied),
            0xE9 => handle_opcode_twobytes!(self, sbc_immediate),
            0xEA => handle_opcode_onebyte!(self, nop_implied),
            0xEC => handle_opcode_threebytes!(self, cpx_absolute),
            0xED => handle_opcode_threebytes!(self, sbc_absolute),
            0xEE => handle_opcode_threebytes!(self, inc_absolute),
            0xF0 => handle_opcode_twobytes!(self, beq),
            0xF1 => handle_opcode_twobytes!(self, sbc_indirect_y),
            0xF5 => handle_opcode_twobytes!(self, sbc_zero_page_x),
            0xF6 => handle_opcode_twobytes!(self, inc_zero_page_x),
            0xF8 => handle_opcode_onebyte!(self, sed),
            0xF9 => handle_opcode_threebytes!(self, sbc_absolute_y),
            0xFD => handle_opcode_threebytes!(self, sbc_absolute_x),
            0xFE => handle_opcode_threebytes!(self, inc_absolute_x),
            0x4B => (0, 2),
            0x0B => (0, 2),
            0x2B => (0, 2),
            0x8B => (0, 2),
            0x6B => (0, 2),
            0xC7 => handle_opcode_twobytes!(self, dcp_zero_page),
            0xD7 => handle_opcode_twobytes!(self, dcp_zero_page_x),
            0xCF => handle_opcode_threebytes!(self, dcp_absolute),
            0xDF => handle_opcode_threebytes!(self, dcp_absolute_x),
            0xDB => handle_opcode_threebytes!(self, dcp_absolute_y),
            0xC3 => handle_opcode_twobytes!(self, dcp_indirect_x),
            0xD3 => handle_opcode_twobytes!(self, dcp_indirect_y),
            0xE7 => handle_opcode_twobytes!(self, isb_zero_page),
            0xF7 => handle_opcode_twobytes!(self, isb_zero_page_x),
            0xEF => handle_opcode_threebytes!(self, isb_absolute),
            0xFF => handle_opcode_threebytes!(self, isb_absolute_x),
            0xFB => handle_opcode_threebytes!(self, isb_absolute_y),
            0xE3 => handle_opcode_twobytes!(self, isb_indirect_x),
            0xF3 => handle_opcode_twobytes!(self, isb_indirect_y),
            0xBB => (0, 3),
            0xA7 => handle_opcode_twobytes!(self, lax_zero_page),
            0xB7 => handle_opcode_twobytes!(self, lax_zero_page_y),
            0xAF => handle_opcode_threebytes!(self, lax_absolute),
            0xBF => handle_opcode_threebytes!(self, lax_absolute_y),
            0xA3 => handle_opcode_twobytes!(self, lax_indirect_x),
            0xB3 => handle_opcode_twobytes!(self, lax_indirect_y),
            0xAB => (0, 2),
            0x27 => handle_opcode_twobytes!(self, rla_zero_page),
            0x37 => handle_opcode_twobytes!(self, rla_zero_page_x),
            0x2F => handle_opcode_threebytes!(self, rla_absolute),
            0x3F => handle_opcode_threebytes!(self, rla_absolute_x),
            0x3B => handle_opcode_threebytes!(self, rla_absolute_y),
            0x23 => handle_opcode_twobytes!(self, rla_indirect_x),
            0x33 => handle_opcode_twobytes!(self, rla_indirect_y),
            0x67 => handle_opcode_twobytes!(self, rra_zero_page),
            0x77 => handle_opcode_twobytes!(self, rra_zero_page_x),
            0x6F => handle_opcode_threebytes!(self, rra_absolute),
            0x7F => handle_opcode_threebytes!(self, rra_absolute_x),
            0x7B => handle_opcode_threebytes!(self, rra_absolute_y),
            0x63 => handle_opcode_twobytes!(self, rra_indirect_x),
            0x73 => handle_opcode_twobytes!(self, rra_indirect_y),
            0x87 => handle_opcode_twobytes!(self, sax_zero_page),
            0x97 => handle_opcode_twobytes!(self, sax_zero_page_y),
            0x8F => handle_opcode_threebytes!(self, sax_absolute),
            0x83 => handle_opcode_twobytes!(self, sax_indirect_x),
            0xCB => (0, 2),
            0x9F => (0, 3),
            0x93 => (0, 2),
            0x9E => (0, 3),
            0x9C => (0, 3),
            0x07 => handle_opcode_twobytes!(self, slo_zero_page),
            0x17 => handle_opcode_twobytes!(self, slo_zero_page_x),
            0x0F => handle_opcode_threebytes!(self, slo_absolute),
            0x1F => handle_opcode_threebytes!(self, slo_absolute_x),
            0x1B => handle_opcode_threebytes!(self, slo_absolute_y),
            0x03 => handle_opcode_twobytes!(self, slo_indirect_x),
            0x13 => handle_opcode_twobytes!(self, slo_indirect_y),
            0x47 => handle_opcode_twobytes!(self, sre_zero_page),
            0x57 => handle_opcode_twobytes!(self, sre_zero_page_x),
            0x4F => handle_opcode_threebytes!(self, sre_absolute),
            0x5F => handle_opcode_threebytes!(self, sre_absolute_x),
            0x5B => handle_opcode_threebytes!(self, sre_absolute_y),
            0x43 => handle_opcode_twobytes!(self, sre_indirect_x),
            0x53 => handle_opcode_twobytes!(self, sre_indirect_y),
            0x9B => (0, 3),
            0xEB => handle_opcode_twobytes!(self, usbc),
            0x1A => (0, 1),
            0x3A => (0, 1),
            0x5A => (0, 1),
            0x7A => (0, 1),
            0xDA => (0, 1),
            0xFA => (0, 1),
            0x80 => (0, 2),
            0x82 => (0, 2),
            0x89 => (0, 2),
            0xC2 => (0, 2),
            0xE2 => (0, 2),
            0x04 => (0, 2),
            0x44 => (0, 2),
            0x64 => (0, 2),
            0x14 => (0, 2),
            0x34 => (0, 2),
            0x54 => (0, 2),
            0x74 => (0, 2),
            0xD4 => (0, 2),
            0xF4 => (0, 2),
            0x0C => (0, 3),
            0x1C => (0, 3),
            0x3C => (0, 3),
            0x5C => (0, 3),
            0x7C => (0, 3),
            0xDC => (0, 3),
            0xFC => (0, 3),
            0x02 => (0, 1),
            0x12 => (0, 1),
            0x22 => (0, 1),
            0x32 => (0, 1),
            0x42 => (0, 1),
            0x52 => (0, 1),
            0x62 => (0, 1),
            0x72 => (0, 1),
            0x92 => (0, 1),
            0xB2 => (0, 1),
            0xD2 => (0, 1),
            0xF2 => (0, 1),
        }
    }

    pub fn tick_ins(&mut self) {
        let opcode = self.memory.fetch_absolute(self.registers.program_counter);
        let old_pc = self.registers.program_counter;
        info!(
            "{:02X}  {:04X}\t\t\tA:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X} PPU:  0, 0 CYC:{}",
            old_pc,
            opcode,
            self.registers.accumulator,
            self.registers.index_x,
            self.registers.index_y,
            self.registers.processor_status,
            self.registers.stack_pointer,
            self.num_cycles
        );
        let (cycles, bytes) = self.decode_execute(opcode);
        self.num_cycles += cycles as usize;
        self.registers.program_counter = self.registers.program_counter.wrapping_add(bytes as u16);
    }

    pub fn tick(&mut self, dur_cycles: usize) {
        let start_cycles = self.num_cycles;
        while self.num_cycles - start_cycles < dur_cycles {
            self.tick_ins();
        }
    }
}
