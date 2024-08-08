pub struct Registers {
    // points to the next instruction to be executed
    pub program_counter: u16,
    // holds low 8 bits of next free location in stack
    pub stack_pointer: u8,
    // used with all arithmetic and logical operations
    pub accumulator: u8,
    // holds counters or offsets for accessing Memory
    // get a copy of the stack pointer or change its value
    // the value of the X register can be loaded and saved in memory,
    // compared with values held in memory or incremented and decremented.
    pub index_x: u8,
    // similar to index_x, although no special functions
    pub index_y: u8,
    // each flag has single bit within register
    // Bit 1: carry flag
    // Bit 2: zero flag
    // Bit 3: interrupt disable
    // Bit 4: decimal mode
    // Bit 5: break command
    // Bit 6: 1
    // Bit 7: overflow flag
    // Bit 8: negative flag
    pub processor_status: u8,
}

impl Registers {
    pub fn new() -> Registers {
        Self {
            program_counter: 0, // will be initialized later after loading ROM
            stack_pointer: 0xfd,
            accumulator: 0,
            index_x: 0,
            index_y: 0,
            processor_status: 0b0010_0100,
        }
    }

    /*
     * Clear bit "setter" methods
     */

    fn unset_nth_status_bit(&mut self, n: u8) {
        self.processor_status &= !(1 << n);
    }

    pub fn unset_carry(&mut self) {
        self.unset_nth_status_bit(0);
    }

    pub fn unset_zero(&mut self) {
        self.unset_nth_status_bit(1);
    }

    pub fn unset_interrupt_disable(&mut self) {
        self.unset_nth_status_bit(2);
    }

    pub fn unset_decimal(&mut self) {
        self.unset_nth_status_bit(3);
    }

    pub fn unset_break(&mut self) {
        self.unset_nth_status_bit(4);
    }

    pub fn unset_overflow(&mut self) {
        self.unset_nth_status_bit(6);
    }

    pub fn unset_neg(&mut self) {
        self.unset_nth_status_bit(7);
    }

    /*
     * Set bit "setter" methods
     */

    fn set_nth_status_bit(&mut self, n: u8) {
        self.processor_status |= 1 << n;
    }

    pub fn set_carry(&mut self) {
        self.set_nth_status_bit(0);
    }

    pub fn set_zero(&mut self) {
        self.set_nth_status_bit(1);
    }

    pub fn set_interrupt_disable(&mut self) {
        self.set_nth_status_bit(2);
    }

    pub fn set_decimal(&mut self) {
        self.set_nth_status_bit(3);
    }

    pub fn set_break(&mut self) {
        self.set_nth_status_bit(4);
    }

    pub fn set_overflow(&mut self) {
        self.set_nth_status_bit(6);
    }

    pub fn set_neg(&mut self) {
        self.set_nth_status_bit(7);
    }

    /*
     * Get bit "getter" methods
     */
    fn get_nth_status_bit(&mut self, n: u8) -> u8 {
        self.processor_status & (1 << n)
    }

    pub fn get_carry(&mut self) -> u8 {
        self.get_nth_status_bit(0)
    }

    pub fn get_zero(&mut self) -> u8 {
        self.get_nth_status_bit(1)
    }

    pub fn get_interrupt_disable(&mut self) -> u8 {
        self.get_nth_status_bit(2)
    }

    pub fn get_decimal(&mut self) -> u8 {
        self.get_nth_status_bit(3)
    }

    pub fn get_break(&mut self) -> u8 {
        self.get_nth_status_bit(4)
    }

    pub fn get_overflow(&mut self) -> u8 {
        self.get_nth_status_bit(6)
    }

    pub fn get_neg(&mut self) -> u8 {
        self.get_nth_status_bit(7)
    }
}
