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
     *   AND - Logical AND operation
     *   Performs a bit by bit AND operation on the accumulator contents using the contents of a byte of memory.
     */

    // Opcode: $29
    // Cycles: 2
    fn and_immediate(&mut self, value: u8){
        self.registers.accumulator &= value;

        if self.registers.accumulator == 0{
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        if self.registers.accumulator & 0b1000_0000 != 0{
            self.registers.set_neg()
        } else {
            self.registers.unset_neg()
        }
    } 

    // Opcode: $25
    // Cycles: 3
    fn and_zero_page(&mut self, addr_lower_byte: u8){
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.and_immediate(value);
    }

    // Opcode: $35
    // Cycles: 4
    fn and_zero_page_x(&mut self, addr_lower_byte: u8){
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        
        self.and_immediate(value);
    }

    // Opcode: $2D
    // Cycles: 4
    fn and_absolute(&mut self, address: u16){
        let value = self.memory.fetch_absolute(address);

        self.and_immediate(value);
    }

    // Opcode: $3D
    // Cycles: 4 (+1 if page crossed)
    fn and_absolute_x(&mut self, address: u16){
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.and_immediate(value);
    }

    // Opcode: $39
    // Cycles: 4 (+1 if page crossed)
    fn and_absolute_y(&mut self, address: u16){
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.and_immediate(value);
    }

    // Opcode: $21
    // Cycles: 6
    fn and_indirect_x(&mut self, addr_lower_byte: u8){
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        
        self.and_immediate(value);
    }

    // Opcode: $31
    // Cycles: 5 (+1 if page crossed)
    fn and_indirect_y(&mut self, addr_lower_byte: u8){
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
    fn eor_immediate(&mut self, value: u8){
        self.registers.accumulator ^= value

        if self.registers.accumulator == 0{
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        if self.registers.accumulator & 0b1000_0000 != 0{
            self.registers.set_neg()
        } else {
            self.registers.unset_neg()
        }
    }

    // Opcode: $45
    // Cycles: 3
    fn eor_zero_page(&mut self, addr_lower_byte: u8){
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.eor_immediate(value);
    }

    // Opcode: $55
    // Cycles: 4
    fn eor_zero_page_x(&mut self, addr_lower_byte: u8){
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        
        self.eor_immediate(value);
    }

    // Opcode: $4D
    // Cycles: 4
    fn eor_absolute(&mut self, address: u16){
        let value = self.memory.fetch_absolute(address);

        self.eor_immediate(value);
    }

    // Opcode: $5D
    // Cycles: 4 (+1 if page crossed)
    fn eor_absolute_x(&mut self, address: u16){
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.eor_immediate(value);
    }

    // Opcode: $59
    // Cycles: 4 (+1 if page crossed)
    fn eor_absolute_y(&mut self, address: u16){
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.eor_immediate(value);
    }

    // Opcode: $41
    // Cycles: 6
    fn eor_indirect_x(&mut self, addr_lower_byte: u8){
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        
        self.eor_immediate(value);
    }

    // Opcode: $51
    // Cycles: 5 (+1 if page crossed)
    fn eor_indirect_y(&mut self, addr_lower_byte: u8){
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
    fn ora_immediate(&mut self, value: u8){
        self.registers.accumulator |= value

        if self.registers.accumulator == 0{
            self.registers.set_zero();
        } else {
            self.registers.unset_zero();
        }

        if self.registers.accumulator & 0b1000_0000 != 0{
            self.registers.set_neg()
        } else {
            self.registers.unset_neg()
        }
    }

    // Opcode: $05
    // Cycles: 3
    fn ora_zero_page(&mut self, addr_lower_byte: u8){
        let value = self.memory.fetch_zero_page(addr_lower_byte);

        self.ora_immediate(value);
    }

    // Opcode: $15
    // Cycles: 4
    fn ora_zero_page_x(&mut self, addr_lower_byte: u8){
        let value = self
            .memory
            .fetch_zero_page_x(addr_lower_byte, self.registers.index_x);
        
        self.ora_immediate(value);
    }

    // Opcode: $0D
    // Cycles: 4
    fn ora_absolute(&mut self, address: u16){
        let value = self.memory.fetch_absolute(address);

        self.ora_immediate(value);
    }

    // Opcode: $1D
    // Cycles: 4 (+1 if page crossed)
    fn ora_absolute_x(&mut self, address: u16){
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_x);

        self.ora_immediate(value);
    }

    // Opcode: $19
    // Cycles: 4 (+1 if page crossed)
    fn ora_absolute_y(&mut self, address: u16){
        let value = self
            .memory
            .fetch_absolute_x(address, self.registers.index_y);

        self.ora_immediate(value);
    }

    // Opcode: $01
    // Cycles: 6
    fn ora_indirect_x(&mut self, addr_lower_byte: u8){
        let value = self
            .memory
            .fetch_indirect_x(addr_lower_byte, self.registers.index_x);
        
        self.ora_immediate(value);
    }

    // Opcode: $11
    // Cycles: 5 (+1 if page crossed)
    fn ora_indirect_y(&mut self, addr_lower_byte: u8){
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
    fn bit_zero_page(&mut self, addr_lower_byte: u8){
        let value = self.memory.fetch_zero_page(addr_lower_byte)
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
    fn bit_absolute(&mut self, address: u16){
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
