/// xines - MOS 6502 instruction set implementation
/// Clock speed: 1.789773 MHz
mod memory;
mod registers;

pub struct Cpu {
    memory: memory::Memory,
    registers: registers::Registers,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            memory: memory::Memory::new(),
            registers: registers::Registers::new(),
        }
    }

    // Helper method
    fn update_zero_negative_flags(&mut self, value: u8) {
        if value == 0 {
            self.registers.set_zero();
        } else {
            self.registers.clear_zero();
        }
        if value & 0b1000_0000 != 0 {
            self.registers.set_neg();
        } else {
            self.registers.clear_neg();
        }
    }

    /*
     * Stack abstraction methods
     */

    fn stack_push(&mut self, val: u8) {
        self.memory.buffer[self.registers.stack_pointer] = val;
        self.registers.stack_pointer += 1;
    }

    fn stack_pop(&mut self) -> u8 {
        let top = self.memory.buffer[self.registers.stack_pointer];
        self.registers.stack_pointer -= 1;
        top
    }

    /*
     * ADC - Add with Carry
     * This instruction adds the contents of a memory location to the accumulator together with the carry bit. If overflow occurs the carry bit is set, this enables multiple byte addition to be performed.
     */

    // Opcode: $69
    // 2 cycles
    fn adc_immediate(&mut self, value: u8) {
        // check if both are positive or if both are negative
        let same_sign = (value & 0b1000_0000) == (self.registers.accumulator & 0b1000_0000);

        let sum: u16 = (self.registers.accumulator as u16) + value + self.registers.get_carry();

        // check if two positive sum to neg or vice versa
        if same_sign && (sum & 0b1000_0000) != (value & (0b1000_0000 as u16)) {
            self.registers.set_overflow()
        }

        // if need to use u16 range, then carry detected
        if sum > 0xFF {
            self.registers.set_carry();
            self.registers.accumulator = (sum & 0b1111_1111) as u8;
        } else {
            self.registers.accumulator = sum as u8;
        }

        // POTENTIAL BUG: set_zero after setting accumulator or before
        if self.registers.accumulator == 0 {
            self.registers.set_zero()
        }

        if self.registers.accumulator & 0b1000_0000 == 1 {
            self.registers.set_neg()
        }
    }

    // Opcode: $65
    // 3 cycles
    fn adc_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.adc_immediate(value)
    }

    // Opcode: $75
    // 4 cycles
    fn adc_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.adc_immediate(value)
    }

    // Opcode: $6D
    // 4 cycles
    fn adc_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.adc_immediate(value)
    }

    // Opcode: $7D
    // 4 (+1 if page crossed) cycles
    fn adc_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.adc_immediate(value)
    }

    // Opcode: $79
    // 4 (+1 if page crossed) cycles
    fn adc_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.adc_immediate(value)
    }

    // Opcode: $61
    // 6 cycles
    fn adc_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.adc_immediate(value)
    }

    // Opcode: $71
    // 5 (+1 if page crossed) cycles
    fn adc_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.adc_immediate(value)
    }

    /*
     * SBC - Subtract with Carry
     * This instruction subtracts the contents of a memory location to the accumulator together with the not of the carry bit. If overflow occurs the carry bit is clear, this enables multiple byte subtraction to be performed.
     */

    // Opcode: $E9
    // 2 cycles
    fn sbc_immediate(&mut self, value: u8) {
        self.adc_immediate((value as i8 * -1i8) as u8) // twos complement
    }

    // Opcode: $E5
    // 3 cycles
    fn sbc_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.sbc_immediate(value)
    }

    // Opcode: $F5
    // 4 cycles
    fn sbc_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.sbc_immediate(value)
    }

    // Opcode: $ED
    // 4 cycles
    fn sbc_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.sbc_immediate(value)
    }

    // Opcode: $FD
    // 4 (+1 if page crossed) cycles
    fn sbc_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.sbc_immediate(value)
    }

    // Opcode: $F9
    // 4 (+1 if page crossed) cycles
    fn sbc_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.sbc_immediate(value)
    }

    // Opcode: $E1
    // 6 cycles
    fn sbc_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.sbc_immediate(value)
    }

    // Opcode: $F1
    // 5 (+1 if page crossed) cycles
    fn sbc_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.sbc_immediate(value)
    }

    /*
     * CMP - Compare
     * This instruction compares the contents of the accumulator with another memory held value and sets the zero and carry flags as appropriate.
     */

    // Opcode: $E9
    // 2 cycles
    fn cmp_immediate(&mut self, value: u8) {
        if (self.registers.accumulator == value) {
            self.registers.set_zero()
        }
        if (self.registers.accumulator >= value) {
            self.registers.set_carry()
        }

        let result = self.registers.accumulator - value;

        // POTENTIAL BUG: do we set bit 7 to neg flag directly or only if neg?
        if (result & 0b1000_0000 == 1) {
            self.registers.set_neg()
        }
    }

    // Opcode: $E5
    // 3 cycles
    fn cmp_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.cmp_immediate(value)
    }

    // Opcode: $F5
    // 4 cycles
    fn cmp_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.cmp_immediate(value)
    }

    // Opcode: $ED
    // 4 cycles
    fn cmp_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.cmp_immediate(value)
    }

    // Opcode: $FD
    // 4 (+1 if page crossed) cycles
    fn cmp_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.cmp_immediate(value)
    }

    // Opcode: $F9
    // 4 (+1 if page crossed) cycles
    fn cmp_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.cmp_immediate(value)
    }

    // Opcode: $E1
    // 6 cycles
    fn cmp_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.cmp_immediate(value)
    }

    // Opcode: $F1
    // 5 (+1 if page crossed) cycles
    fn cmp_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.cmp_immediate(value)
    }

    /*
     * CPX - Compare X Register
     * This instruction compares the contents of the X register with another memory held value and sets the zero and carry flags as appropriate.
     */

    // Opcode: $E0
    // 2 cycles
    fn cpx_immediate(&mut self, value: u8) {
        if (self.registers.index_x == value) {
            self.registers.set_zero()
        }
        if (self.registers.index_x >= value) {
            self.registers.set_carry()
        }

        let result = self.registers.index_x - value;

        // POTENTIAL BUG: do we set bit 7 to neg flag directly or only if neg?
        if (result & 0b1000_0000 == 1) {
            self.registers.set_neg()
        }
    }

    // Opcode: $E4
    // 3 cycles
    fn cpx_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.cpx_immediate(value)
    }

    // Opcode: $EC
    // 4 cycles
    fn cpx_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.cpx_immediate(value)
    }

    /*
     * CPY - Compare Y Register
     * This instruction compares the contents of the Y register with another memory held value and sets the zero and carry flags as appropriate.
     */

    // Opcode: $C0
    // 2 cycles
    fn cpy_immediate(&mut self, value: u8) {
        if (self.registers.index_y == value) {
            self.registers.set_zero()
        }
        if (self.registers.index_y >= value) {
            self.registers.set_carry()
        }

        let result = self.registers.index_y - value;

        // POTENTIAL BUG: do we set bit 7 to neg flag directly or only if neg?
        if (result & 0b1000_0000 == 1) {
            self.registers.set_neg()
        }
    }

    // Opcode: $C4
    // 3 cycles
    fn cpy_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.cpy_immediate(value)
    }

    // Opcode: $CC
    // 4 cycles
    fn cpy_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.cpy_immediate(value)
    }

    /*
     * ASL - Arithmetic Shift Left
     * This operation shifts all the bits of the accumulator or memory contents one bit left. Bit 0 is set to 0 and bit 7 is placed in the carry flag. The effect of this operation is to multiply the memory contents by 2 (ignoring 2's complement considerations), setting the carry if the result will not fit in 8 bits.
     */

    // Helper method to extract general ASL functionality
    fn asl_immediate(&mut self, value: u8) -> u8 {
        // Bit 7 is set in carry flag
        let first_bit = value & 0b1000_0000;
        if first_bit == 1 {
            self.registers.set_carry()
        }

        let new_value = value << 1;

        self.update_zero_negative_flags(new_value);

        new_value
    }

    // Opcode: $0A
    // 2 cycles
    fn asl_accumulator(&mut self) {
        let new_accum = self.asl_immediate(self.registers.accumulator);
        self.registers.accumulator = new_accum;
    }

    // Opcode: $06
    // 5 cycles
    fn asl_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.asl_immediate(value);
        // store value in same memory address
    }

    // Opcode: $16
    // 6 cycles
    fn asl_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.asl_immediate(value);
    }

    // Opcode: $0E
    // 6 cycles
    fn asl_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        let value = self.asl_immediate(value);
    }

    // Opcode: $1E
    // 7 cycles
    fn asl_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.asl_immediate(value);
    }

    /*
     * LSR - Logical Shift Right
     * Each of the bits in A or M is shift one place to the right. The bit that was in bit 0 is shifted into the carry flag. Bit 7 is set to zero.
     */

    // Helper method to extract general ASL functionality
    fn lsr_immediate(&mut self, value: u8) -> u8 {
        // Bit 7 is set in carry flag
        let first_bit = value & 0b0000_0001;
        if first_bit == 1 {
            self.registers.set_carry()
        }

        let new_value = value >> 1;

        if (new_value == 0) {
            self.registers.set_zero();
        }

        // Not really necessary as bit 7 will be 0
        /* if (new_value & 0b1000_000 == 1) {
            self.registers.set_neg()
        } */

        new_value
    }

    // Opcode: $4A
    // 2 cycles
    fn lsr_accumulator(&mut self) {
        let new_accum = self.lsr_immediate(self.registers.accumulator);
        self.registers.accumulator = new_accum;
    }

    // Opcode: $46
    // 5 cycles
    fn lsr_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.lsr_immediate(value);
        // store value in same memory address
    }

    // Opcode: $56
    // 6 cycles
    fn lsr_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.lsr_immediate(value);
    }

    // Opcode: $4E
    // 6 cycles
    fn lsr_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        let value = self.lsr_immediate(value);
    }

    // Opcode: $5E
    // 7 cycles
    fn lsr_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.lsr_immediate(value);
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

        if first_bit == 1 {
            self.registers.set_carry()
        }

        self.update_zero_negative_flags(new_value);
        new_value
    }

    // Opcode: $2A
    // 2 cycles
    fn rol_accumulator(&mut self) {
        self.registers.accumulator = self.rol_immediate(self.registers.accumulator);
    }

    // Opcode: $26
    // 5 cycles
    fn rol_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.rol_immediate(value);
        // store value in same memory address
    }

    // Opcode: $36
    // 6 cycles
    fn rol_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.rol_immediate(value);
    }

    // Opcode: $2E
    // 6 cycles
    fn rol_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        let value = self.rol_immediate(value);
    }

    // Opcode: $3E
    // 7 cycles
    fn rol_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.rol_immediate(value);
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

        if last_bit == 1 {
            self.registers.set_carry()
        }

        self.update_zero_negative_flags(new_value);

        new_value
    }

    // Opcode: $6A
    // 2 cycles
    fn ror_accumulator(&mut self) {
        self.registers.accumulator = self.ror_immediate(self.registers.accumulator);
    }

    // Opcode: $66
    // 5 cycles
    fn ror_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let value = self.ror_immediate(value);
        // store value in same memory address
    }

    // Opcode: $76
    // 6 cycles
    fn ror_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        let value = self.ror_immediate(value);
    }

    // Opcode: $6E
    // 6 cycles
    fn ror_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        let value = self.ror_immediate(value);
    }

    // Opcode: $7E
    // 7 cycles
    fn ror_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        let value = self.ror_immediate(value);
    }

    /*
     * LDA - Load accumulator
     * Loads a byte of memory into the accumulator setting the zero and negative flags as appropriate.
     */

    // Opcode: $A9
    // 2 cycles
    fn lda_immediate(&mut self, value: u8) {
        self.registers.accumulator = value;
        // TODO: confirm if AFTER
        self.update_zero_negative_flags(self.registers.accumulator);
    }

    // Opcode: $AD
    // 4 cycles
    fn lda_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.lda_immediate(value)
    }

    // Opcode: $A5
    // 3 cycles
    fn lda_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.lda_immediate(value)
    }

    // Opcode: $B5
    // 4 cycles
    fn lda_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.lda_immediate(value)
    }

    // Opcode: $BD
    // 4 (+1 if page crossed) cycles
    fn lda_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.lda_immediate(value)
    }

    // Opcode: $B9
    // 4 (+1 if page crossed) cycles
    fn lda_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.lda_immediate(value)
    }

    // Opcode: $A1
    // 6 cycles
    fn lda_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.lda_immediate(value)
    }

    // Opcode: $B1
    // 5 (+1 if page crossed) cycles
    fn lda_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.lda_immediate(value)
    }

    /*
     *
     * LDX - Load X Register
     * Loads a byte of memory into the X register setting the zero and negative flags as appropriate.
     *
     */

    // Opcode: $A2
    // 2 cycles
    fn ldx_immediate(&mut self, value: u8) {
        self.registers.index_x = value;
        self.update_zero_negative_flags(self.registers.index_x);
    }

    // Opcode: $AE
    // 4 cycles
    fn ldx_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.ldx_immediate(value)
    }

    // Opcode: $BE
    // 4 (+1 if page crossed) cycles
    fn ldx_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.ldx_immediate(value)
    }

    // Opcode: $A6
    // 3 cycles
    fn ldx_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.ldx_immediate(value)
    }

    // Opcode: $B6
    // 4 cycles
    fn ldx_zero_page_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_y);
        self.ldx_immediate(value)
    }

    /*
     *
     * LDY - Load Y Register
     * Loads a byte of memory into the Y register setting the zero and negative flags as appropriate.
     *
     */

    // Opcode: $A0
    // 2 cycles
    fn ldy_immediate(&mut self, value: u8) {
        self.registers.index_y = value;
        self.update_zero_negative_flags(self.registers.index_y);
    }

    // Opcode: $AC
    // 4 cycles
    fn ldy_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        self.ldy_immediate(value)
    }

    // Opcode: $BC
    // 4 (+1 if page crossed) cycles
    fn ldy_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.ldy_immediate(value)
    }

    // Opcode: $A4
    // 3 cycles
    fn ldy_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        self.ldy_immediate(value)
    }

    // Opcode: $B4
    // 4 cycles
    fn ldy_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        self.ldy_immediate(value)
    }

    /*
     *   TAX - Transfer accumulator to X
     *   Copies the current contents of the accumulator into the X register and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $AA
     *   Cycles: 2
     */
    fn tax(&mut self) {
        self.ldx_immediate(self.registers.accumulator);
    }

    /*
     *   TAY - Transfer Accumulator to Y
     *   Copies the current contents of the accumulator into the Y register and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $A8
     *   Cycles: 2
     */
    fn tay(&mut self) {
        self.ldy_immediate(self.registers.accumulator);
    }

    /*
     *   TSX - Transfer Stack Pointer to X
     *   Copies the current contents of the stack register into the X register and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $BA
     *   Cycles: 2
     */
    fn tsx(&mut self) {
        self.ldx_immediate(self.registers.stack_pointer)
    }

    /*
     *   TXA - Transfer X to accumulator
     *   Copies the current contents of the X register into the accumulator and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $8A
     *   Cycles: 2
     */
    fn txa(&mut self) {
        self.lda_immediate(self.registers.index_x)
    }

    /*
     *   TXS - Transfer X to Stack Pointer
     *   Copies the current contents of the X register into the stack register.
     *
     *   Opcode: $9A
     *   Cycles: 2
     */
    fn txs(&mut self) {
        self.registers.stack_pointer = self.registers.index_x;
    }

    /*
     *   TYA - Transfer Y to Accumulator
     *   Copies the current contents of the Y register into the accumulator and sets the zero and negative flags as appropriate.
     *
     *   Opcode: $98
     *   Cycles: 2
     */
    fn tya(&mut self) {
        self.lda_immediate(self.registers.index_y)
    }

    /*
     *   AND - Logical AND operation
     *   Performs a bit by bit AND operation on the accumulator contents using the contents of a byte of memory.
     */

    // Opcode: $29
    // Cycles: 2
    fn and_immediate(&mut self, value: u8) {
        self.registers.accumulator &= value;

        self.update_zero_negative_flags(self.registers.accumulator);
    }

    // Opcode: $25
    // Cycles: 3
    fn and_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.and_immediate(value);
    }

    // Opcode: $35
    // Cycles: 4
    fn and_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);

        self.and_immediate(value);
    }

    // Opcode: $2D
    // Cycles: 4
    fn and_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);

        self.and_immediate(value);
    }

    // Opcode: $3D
    // Cycles: 4 (+1 if page crossed)
    fn and_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.and_immediate(value);
    }

    // Opcode: $39
    // Cycles: 4 (+1 if page crossed)
    fn and_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.and_immediate(value);
    }

    // Opcode: $21
    // Cycles: 6
    fn and_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);

        self.and_immediate(value);
    }

    // Opcode: $31
    // Cycles: 5 (+1 if page crossed)
    fn and_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_x);

        self.and_immediate(value);
    }

    /*
     *   EOR - Exclusive OR
     *   Perform the XOR operation
     */

    // Opcode: $49
    // Cycles: 2
    fn eor_immediate(&mut self, value: u8) {
        self.registers.accumulator ^= value;

        self.update_zero_negative_flags(self.registers.accumulator);
    }

    // Opcode: $45
    // Cycles: 3
    fn eor_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.eor_immediate(value);
    }

    // Opcode: $55
    // Cycles: 4
    fn eor_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);

        self.eor_immediate(value);
    }

    // Opcode: $4D
    // Cycles: 4
    fn eor_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);

        self.eor_immediate(value);
    }

    // Opcode: $5D
    // Cycles: 4 (+1 if page crossed)
    fn eor_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.eor_immediate(value);
    }

    // Opcode: $59
    // Cycles: 4 (+1 if page crossed)
    fn eor_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.eor_immediate(value);
    }

    // Opcode: $41
    // Cycles: 6
    fn eor_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);

        self.eor_immediate(value);
    }

    // Opcode: $51
    // Cycles: 5 (+1 if page crossed)
    fn eor_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_x);

        self.eor_immediate(value);
    }

    /*
     *   ORA - Logical OR Operation
     *   Perform the logical OR operation
     */

    // Opcode: $09
    // Cycles: 2
    fn ora_immediate(&mut self, value: u8) {
        self.registers.accumulator |= value;

        self.update_zero_negative_flags(self.registers.accumulator);
    }

    // Opcode: $05
    // Cycles: 3
    fn ora_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.ora_immediate(value);
    }

    // Opcode: $15
    // Cycles: 4
    fn ora_zero_page_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);

        self.ora_immediate(value);
    }

    // Opcode: $0D
    // Cycles: 4
    fn ora_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);

        self.ora_immediate(value);
    }

    // Opcode: $1D
    // Cycles: 4 (+1 if page crossed)
    fn ora_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.ora_immediate(value);
    }

    // Opcode: $19
    // Cycles: 4 (+1 if page crossed)
    fn ora_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.ora_immediate(value);
    }

    // Opcode: $01
    // Cycles: 6
    fn ora_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);

        self.ora_immediate(value);
    }

    // Opcode: $11
    // Cycles: 5 (+1 if page crossed)
    fn ora_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_x);

        self.ora_immediate(value);
    }

    /*
     *   BIT - BIT Test for certain bits
     *   Check if one or more bits are set in target memory location
     */

    // Opcode: $24
    // Cycles: 3
    fn bit_zero_page(&mut self, addr_lower_byte: u8) {
        let value = self.memory.fetch_zero_page(addr_lower_byte);
        let result = self.registers.accumulator & value;

        if result == 0 {
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        if (value & 0b1000_0000) != 0 {
            self.registers.set_neg();
        } else {
            self.registers.unset_neg();
        }

        if (value & 0b0100_0000) != 0 {
            self.registers.set_overflow();
        } else {
            self.registers.unset_overflow();
        }
    }

    // Opcode: $2C
    // Cycles: 4
    fn bit_absolute(&mut self, address: u16) {
        let value = self.memory.fetch_absolute(address);
        let result = self.registers.accumulator & value;

        if result == 0 {
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        if (value & 0b1000_0000) != 0 {
            self.registers.set_neg();
        } else {
            self.registers.unset_neg();
        }

        if (value & 0b0100_0000) != 0 {
            self.registers.set_overflow();
        } else {
            self.registers.unset_overflow();
        }
    }

    /*
     *   CLC - Clear Carry Flag
     *   Set the carry flag to zero.
     *
     *   Opcode: $18
     *   Cycles: 2
     */
    fn clc(&mut self) {
        self.registers.unset_carry()
    }

    /*
     *   UNUSED FOR NES!
     *   CLD - Clear Decimal Mode
     *   Sets the decimal mode flag to zero.
     *
     *   Opcode: $D8
     *   Cycles: 2
     */
    /* fn cld(&mut self) {
        self.registers.unset_decimal_mode()
    } */

    /*
     *   CLI - Clear Interrupt Disable
     *   Clears the interrupt disable flag allowing normal interrupt requests to be serviced.
     *
     *   Opcode: $58
     *   Cycles: 2
     */
    fn cli(&mut self) {
        self.registers.unset_interrupt_disable()
    }

    /*
     *   CLV - Clear Overflow Flag
     *   Clears the overflow flag.
     *
     *   Opcode: $B8
     *   Cycles: 2
     */
    fn clv(&mut self) {
        self.registers.unset_overflow()
    }

    /*
     *   SEC - Set Carry Flag
     *   Set the carry flag to one.
     *
     *   Opcode: $38
     *   Cycles: 2
     */
    fn sec(&mut self) {
        self.registers.set_carry()
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
    /* fn sed(&mut self) {
        self.registers.set_decimal_mode()
    } */

    /*
     *   SEI - Set Interrupt Disable
     *   Set the interrupt disable flag to one.
     *
     *   Opcode: $78
     *   Cycles: 2
     */
    fn sei(&mut self) {
        self.registers.set_interrupt_disable()
    }

    /*
     *   PHA - Push accumulator
     *   Pushes a copy of the accumulator on to the stack.
     *
     *   Opcode: $48
     *   Cycles: 3
     */
    fn pha(&mut self) {
        self.stack_push(self.registers.accumulator)
    }

    /*
     *   PHP - Push Processor Status
     *   Pushes a copy of the status flags on to the stack.
     *
     *   Opcode: $08
     *   Cycles: 3
     */
    fn php(&mut self) {
        self.stack_push(self.registers.processor_status)
    }

    /*
     *   PLA - Pull Accumulator
     *   Pulls an 8 bit value from the stack and into the accumulator. The zero and negative flags are set as appropriate.
     *
     *   Opcode: $68
     *   Cycles: 4
     */
    fn pla(&mut self) {
        let val = self.stack_pop();
        self.lda_immediate(val);
    }

    /*
     *   PLP - Pull Processor Status
     *   Pulls an 8 bit value from the stack and into the processor flags. The flags will take on new states as determined by the value pulled.
     *
     *   Opcode: $28
     *   Cycles: 4
     */
    fn plp(&mut self) {
        let val = self.stack_pop();
        self.registers.processor_status = val;
    }

    /*
     *   JMP - Jump
     *   Sets the program counter to the address specified by the operand.
     *   For compatibility always ensure the indirect vector is not at the end of the page.
     */

    // Opcode: $4C
    // Cycles: 3
    fn jmp_absolute(&mut self, address: u16) {
        self.registers.program_counter = address;
    }

    // Opcode: $6C
    // Cycles: 5
    fn jmp_indirect(&mut self, address: u16) {
        self.registers.program_counter = self.memory.fetch_indirect(address);
    }

    /*
     *   JSR - Jump to Subroutine
     *   The JSR instruction pushes the address (minus one) of the return point on to the stack and then sets the program counter to the target memory address.
     *
     *   Opcode: $20
     *   Cycles: 6
     */
    fn jsr(&mut self, address: u16) {
        self.stack_push(self.registers.program_counter + (2 as u8));
        self.registers.program_counter = address;
    }

    /*
     *   BCC - Branch if Carry Clear
     *   If the carry flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $90
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bcc(&mut self, offset: u8) {
        if self.registers.get_carry() == 0 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   BCS - Branch if Carry Set
     *   If the carry flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $B0
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bcs(&mut self, offset: u8) {
        if self.registers.get_carry() == 1 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   BEQ - Branch if Equal
     *   If the zero flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $F0
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn beq(&mut self, offset: u8) {
        if self.registers.get_zero() == 1 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   BMI - Branch if Minus
     *   If the negative flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $30
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bmi(&mut self, offset: u8) {
        if self.registers.get_neg() == 1 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   BNE - Branch if Not Equal
     *   If the zero flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $D0
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bne(&mut self, offset: u8) {
        if self.registers.get_zero() == 0 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   BPL - Branch if Positive
     *   If the negative flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $10
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bpl(&mut self, offset: u8) {
        if self.registers.get_neg() == 0 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   BVC - Branch if Overflow Clear
     *   If the overflow flag is clear then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $50
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bvc(&mut self, offset: u8) {
        if self.registers.get_overflow() == 0 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   BVS - Branch if Overflow Set
     *   If the overflow flag is set then add the relative displacement to the program counter to cause a branch to a new location.
     *
     *   Opcode: $70
     *   Cycles: 2 (+1 if branch succeeds +2 if to a new page)
     */
    fn bvs(&mut self, offset: u8) {
        if self.registers.get_overflow() == 1 {
            self.registers.program_counter += offset - 2;
        }
    }

    /*
     *   STA - Store Accumulator
     *   Store the value of the accumulator into memory
     */
    
    // Opcode: $85
    // Cycles: 3
    fn sta_zero_page(&mut self, addr_lower_byte: u8) {
        self.memory.store_zero_page(addr_lower_byte, self.registers.accumulator);
    }

    // Opcode: $95
    // Cycles: 4
    fn sta_zero_page_x(&mut self, addr_lower_byte: u8){
        self.memory.store_zero_page_x(addr_lower_byte, self.registers.index_x, self.registers.accumulator);
    }

    // Opcode: $8D
    // Cycles: 4
    fn sta_absolute(&mut self, address: u16){
        self.memory.store_absolute(address, self.registers.accumulator);
    }

    // Opcode: $9D
    // Cycles: 5
    fn sta_absolute_x(&mut self, address: u16){
        self.store_absolute_x(address, self.registers.index_x, self.registers.accumulator);
    }

    // Opcode: $99
    // Cycles: 5
    fn sta_absolute_y(&mut self, address: u16){
        self.store_absolute_x(address, self.registers.index_y, self.registers.accumulator);
    }

    // Opcode: $81
    // Cycles: 6
    fn sta_indirect_x(&mut self, addr_lower_byte: u8){
        self.store_indirect_x(addr_lower_byte, self.registers.index_x, self.registers.accumulator);
    }

    // Opcode: $91
    // Cycles: 6
    fn sta_indirect_y(&mut self, addr_lower_byte: u8){
        self.store_indirect_x(addr_lower_byte, self.registers.index_y, self.registers.accumulator);
    }

    /*
     *   STX - Store the value at the X register
     *   Store the value of the X register into memory
     */
    
    // Opcode: $86
    // Cycles: 3
    fn stx_zero_page(&mut self, addr_lower_byte: u8) {
        self.memory.store_zero_page(addr_lower_byte, self.registers.index_x);
    }

    // Opcode: $96
    // Cycles: 4
    fn stx_zero_page_x(&mut self, addr_lower_byte: u8) {
        self.memory.store_zero_page_y(addr_lower_byte, self.registers.index_y, self.registers.index_x);
    }

    // Opcode: $8E
    // Cycles: 4
    fn stx_absolute(&mut self, address: u16){
        self.memory.store_absolute(address, self.registers.index_x)
    }

    /*
     *   STY - Store the value at the Y register
     *   Store the value of the Y register into memory
     */
    
    // Opcode: $84
    // Cycles: 3
    fn sty_zero_page(&mut self, addr_lower_byte: u8) {
        self.memory.store_zero_page(addr_lower_byte, self.registers.index_y);
    }

    // Opcode: $94
    // Cycles: 4
    fn sty_zero_page_x(&mut self, addr_lower_byte: u8) {
        self.memory.store_zero_page_x(addr_lower_byte, self.registers.index_x, self.registers.index_y);
    }

    // Opcode: $9C
    // Cycles: 4
    fn sty_absolute(&mut self, address: u16){
        self.memory.store_absolute(address, self.registers.index_y)
    }

    /*
     *   INC - Increment Memory
     *   Increment the value at a specified memory location
     */
    
    // Opcode: $E6
    // Cycles: 5
    fn inc_zero_page(&mut self, addr_lower_byte: u8){
        let new_val = self.memory.fetch_zero_page(addr_lower_byte)+1;

        self.memory.store_zero_page(address, new_val);
        update_zero_negative_flags(new_val);
    }

    // Opcode: $F6
    // Cycles: 6
    fn inc_zero_page_x(&mut self, addr_lower_byte: u8){
        let new_val = self.memory.fetch_zero_page_x(addr_lower_byte, self.registers.index_x)+1;

        self.memory.store_zero_page_x(address, self.registers.index_x, new_val);
        update_zero_negative_flags(new_val);
    }

    // Opcode: $EE
    // Cycles: 6
    fn inc_absolute(&mut self, address: u16) {
        let new_val = self.memory.fetch_absolute(address)+1;

        self.memory.store_absolute(address, new_val);
        update_zero_negative_flags(new_val);
    }

    // Opcode: $FE
    // Cycles: 7
    fn inc_absolute_x(&mut self, address: u16){
        let new_val = self.memory.fetch_absolute_x(address)+1;

        self.memory.store_absolute_x(address, self.registers.index_x, new_val);
        update_zero_negative_flags(new_val);
    }

    /*
     *   INX - Increment X Register
     *   Increment the value at the X Register
     *   
     *   Opcode: $E8
     *   Cycles: 2
     */

    fn inx_implied(&mut self){
        self.registers.index_x += 1
        update_zero_negative_flags(self.registers.index_x);
    }

    /*
     *   INY - Increment Y Register
     *   Increment the value at the Y Register
     *   
     *   Opcode: $C8
     *   Cycles: 2
     */

    fn inx_implied(&mut self){
        self.registers.index_y += 1
        update_zero_negative_flags(self.registers.index_y);
    }





    -----

    /*
     *   DEC - Decrement Memory
     *   Decrement the value at a specified memory location
     */
    
    // Opcode: $C6
    // Cycles: 5
    fn dec_zero_page(&mut self, addr_lower_byte: u8){
        let new_val = self.memory.fetch_zero_page(addr_lower_byte)-1;

        self.memory.store_zero_page(address, new_val);
        update_zero_negative_flags(new_val);
    }

    // Opcode: $D6
    // Cycles: 6
    fn dnc_zero_page_x(&mut self, addr_lower_byte: u8){
        let new_val = self.memory.fetch_zero_page_x(addr_lower_byte, self.registers.index_x)-1;

        self.memory.store_zero_page_x(address, self.registers.index_x, new_val);
        update_zero_negative_flags(new_val);
    }

    // Opcode: $CE
    // Cycles: 6
    fn dec_absolute(&mut self, address: u16) {
        let new_val = self.memory.fetch_absolute(address)-1;

        self.memory.store_absolute(address, new_val);
        update_zero_negative_flags(new_val);
    }

    // Opcode: $DE
    // Cycles: 7
    fn inc_absolute_x(&mut self, address: u16){
        let new_val = self.memory.fetch_absolute_x(address)-1;

        self.memory.store_absolute_x(address, self.registers.index_x, new_val);
        update_zero_negative_flags(new_val);
    }

    /*
     *   DEX - Decrement X Register
     *   Increment the value at the X Register
     *   
     *   Opcode: $CA
     *   Cycles: 2
     */

    fn dex_implied(&mut self){
        self.registers.index_x -= 1
        update_zero_negative_flags(self.registers.index_x);
    }

    /*
     *   DEY - Decrement Y Register
     *   Decrement the value at the Y Register
     *   
     *   Opcode: $88
     *   Cycles: 2
     */

    fn dey_implied(&mut self){
        self.registers.index_y -= 1
        update_zero_negative_flags(self.registers.index_y);
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_cpu() {
        assert_eq!(((0x80 as u8).wrapping_add(0xFF as u8)), 0x7F);
    }
}

fn main() {
    println!("Hello, world!");
}
