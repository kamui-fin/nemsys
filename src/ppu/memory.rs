// The PPU addresses a 14-bit (16kB) address space, $0000-$3FFF, completely separate from the CPU's address bus.
// It is either directly accessed by the PPU itself, or via the CPU with memory mapped registers at $2006 and $2007.

/// Address range 	Size 	Description                 Mapped by
/// $0000-$0FFF 	$1000 	Pattern table 0 	        Cartridge
/// $1000-$1FFF 	$1000 	Pattern table 1 	        Cartridge
/// $2000-$23BF 	$0400 	Nametable 0 	            Cartridge
/// $2400-$27FF 	$0400 	Nametable 1 	            Cartridge
/// $2800-$2BFF 	$0400 	Nametable 2 	            Cartridge
/// $2C00-$2FFF 	$0400 	Nametable 3 	            Cartridge
/// $3000-$3EFF 	$0F00 	Unused 	                    Cartridge
/// $3F00-$3F1F 	$0020 	Palette RAM indexes 	    Internal to PPU
/// $3F20-$3FFF 	$00E0 	Mirrors of $3F00-$3F1F 	    Internal to PPU

/// Hardware mapping

/// The NES has 2kB of RAM dedicated to the PPU, usually mapped to the nametable address space from $2000-$2FFF, but this can be rerouted through custom cartridge wiring.
//  The mappings above are the addresses from which the PPU uses to fetch data during rendering. The actual devices that the PPU fetches pattern, name table and attribute table data from is configured by the cartridge.
/// $0000-1FFF is normally mapped by the cartridge to a CHR-ROM or CHR-RAM, often with a bank switching mechanism.
/// $2000-2FFF is normally mapped to the 2kB NES internal VRAM, providing 2 nametables with a mirroring configuration controlled by the cartridge, but it can be partly or fully remapped to ROM or RAM on the cartridge, allowing up to 4 simultaneous nametables.
/// $3000-3EFF is usually a mirror of the 2kB region from $2000-2EFF. The PPU does not render from this address range, so this space has negligible utility.
/// $3F00-3FFF is not configurable, always mapped to the internal palette control.

pub(crate) struct VRAM {
    pub buffer: [u8; 0x4000],
}

impl VRAM {
    pub fn new() -> Self {
        Self {
            buffer: [0; 0x4000],
        }
    }

    pub fn copy_into_memory(&mut self, buffer: &[u8], starting_address: usize){
        for (i, &value) in buffer.iter().enumerate(){
            let curr_addr = starting_address+i;
            if curr_addr < self.buffer.len(){
                self.buffer[curr_addr] = value;
            }
        }
    }

    pub fn get(&mut self, address: usize) -> u8{
        self.buffer[address]
    }

    pub fn set(&mut self, address: usize, value: u8){
        self.buffer[address] = value;
    }

    pub fn write_callback(&mut self, address: usize, value: u8){
        
    }
}
