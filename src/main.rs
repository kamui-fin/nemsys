/// xines - MOS 6502 instruction set implementation
/// Clock speed: 1.789773 MHz
mod memory;
mod registers;

pub struct Cpu {
    memory: memory::Memory,
    registers: registers::Registers,
}

/*
* Load/Store Operations
*
* These instructions transfer a single byte between memory and one of the registers.
* Load operations set the negative (N) and zero (Z) flags depending on the value of transferred.
* Store operations do not affect the flag settings.
*
* LDA 	Load Accumulator 	N,Z
* LDX 	Load X Register 	N,Z
* LDY 	Load Y Register 	N,Z
* STA 	Store Accumulator
* STX 	Store X Register
* STY 	Store Y Register
*/

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
    fn lda_absolute_x(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);
        self.lda_immediate(value)
    }

    // Opcode: $B9
    fn lda_absolute_y(&mut self, address: u16) {
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);
        self.lda_immediate(value)
    }

    // Opcode: $A1
    fn lda_indirect_x(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        self.lda_immediate(value)
    }

    // Opcode: $B2
    fn lda_indirect_y(&mut self, addr_lower_byte: u8) {
        let value = self
            .memory
            .fetch_indirect_y(addr_lower_byte, self.registers.index_y);
        self.lda_immediate(value)
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
