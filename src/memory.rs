// Memory abstraction layer, acts as the data and address bus
/// 16-bit address bus
/// Special notes:
/// - $0000-$00FF reserved "zero page"
/// - $0100-$01FF reserved for stack
/// - $FFFA to $FFFF reserved
/// - Little endian
// Includes stack abstraction methods
pub struct Memory {
    pub buffer: [u8; 0xFFFF],
}

impl Memory {
    pub fn new() -> Self {
        Self {
            buffer: [0; 0xFFFF],
        }
    }

    pub(crate) fn fetch_absolute(&self, address: u16) -> u8 {
        self.buffer[address as usize]
    }

    pub(crate) fn store_absolute(&self, address: u16, value: u8) {
        self.buffer[address as usize] = value
    }

    // also called for absolute_y
    pub(crate) fn fetch_absolute_x(&self, address: u16, index_x: u8) -> u8 {
        self.fetch_absolute(address + index_x)
    }

    // also called for absolute_y
    pub(crate) fn store_absolute_x(&self, address: u16, index_x: u8, value: u8) -> u8 {
        self.store_absolute(address + index_x, value)
    }

    pub(crate) fn fetch_zero_page(&self, addr_lower_byte: u8) -> u8 {
        self.fetch_absolute(addr_lower_byte as u16)
    }

    pub(crate) fn store_zero_page(&self, addr_lower_byte: u8, value: u8){
        self.store_absolute(addr_lower_byte as u16, value)
    }

    // also called for zero_page_y
    pub(crate) fn fetch_zero_page_x(&self, addr_lower_byte: u8, index_x: u8) -> u8 {
        let addr = addr_lower_byte.wrapping_add(index_x);
        self.fetch_zero_page(addr)
    }

    pub(crate) fn store_zero_page_x(&self, addr_lower_byte: u8, x: u8, value: u8){
        let addr = addr_lower_byte.wrapping_add(x)
        self.store_absolute(addr, value)
    }

    pub(crate) fn fetch_indirect(&self, address: u16) -> u16 {
        (self.fetch_absolute(address) + self.fetch_absolute(address + 1) * 256) as u16
    }

    pub(crate) fn fetch_indirect_x(&self, addr_lower_byte: u8, index_x: u8) -> u8 {
        // val = PEEK(PEEK((arg + X) % 256) + PEEK((arg + X + 1) % 256) * 256)
        let addr = self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x))
            + self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x + 1)) * 256;
        self.fetch_absolute(addr as u16)
    }

    pub(create) fn store_indirect_x(&self, addr_lower_byte: u8, index_x: u8, value: u8) {
        let addr = self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x))
            + self.fetch_zero_page(addr_lower_byte.wrapping_add(index_x + 1)) * 256;
        self.store_absolute(addr as u16, value)
    }

    pub(crate) fn fetch_indirect_y(&self, addr_lower_byte: u8, index_y: u8) -> u8 {
        // val = PEEK(PEEK(arg) + PEEK((arg + 1) % 256) * 256 + Y)
        let addr = self.fetch_zero_page(
            self.fetch_zero_page(addr_lower_byte)
                + self.fetch_zero_page(addr_lower_byte.wrapping_add(1)) * 256
                + index_y,
        );
        self.fetch_absolute(addr as u16)
    }

    pub(crate) fn store_indirect_y(&self, addr_lower_byte: u8, index_y: u8, value: u8) -> u8 {
        // val = PEEK(PEEK(arg) + PEEK((arg + 1) % 256) * 256 + Y)
        let addr = self.fetch_zero_page(
            self.fetch_zero_page(addr_lower_byte)
                + self.fetch_zero_page(addr_lower_byte.wrapping_add(1)) * 256
                + index_y,
        );
        self.store_absolute(addr as u16)
    }
}
