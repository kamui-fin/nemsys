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

    /*
     * LDA - Load accumulator
     * Loads a byte of memory into the accumulator setting the zero and negative flags as appropriate.
     */

    // Opcode: $A9
    // 2 cycles
    fn lda_immediate(&mut self, value: u8) {
        self.registers.accumulator = value;
        // TODO: confirm if AFTER
        if (self.registers.accumulator == 0) {
            self.registers.set_zero();
        }
        if (self.registers.accumulator & 0b1000_0000 == 1) {
            self.registers.set_neg();
        }
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
        if (self.registers.index_x == 0) {
            self.registers.set_zero();
        }
        if (self.registers.index_x & 0b1000_0000 == 1) {
            self.registers.set_neg();
        }
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
        if (self.registers.index_y == 0) {
            self.registers.set_zero();
        }
        if (self.registers.index_y & 0b1000_0000 == 1) {
            self.registers.set_neg();
        }
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
     *   CLC - Clear Carry Flag
     *   Set the carry flag to zero.
     *
     *   Opcode: $18
     *   Cycles: 2
     */
    fn clc(&mut self) {
        self.registers.set_carry(0);
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
