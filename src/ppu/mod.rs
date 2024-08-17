#[cfg(target_family = "wasm")]
pub mod emscripten;
pub(crate) mod memory;
use memory::VRAM;

/// The OAM (Object Attribute Memory) is internal memory inside the PPU that contains a display list of up to 64 sprites, where each sprite's information occupies 4 bytes.
/// Byte 0: Y position of top of sprite
/// Byte 1: Tile index number
/// Byte 2: Attributes
/// Byte 3: X position of left side of sprite.
/// 
/// Most programs write to a copy of OAM somewhere in CPU addressable RAM (often $0200-$02FF) and then copy it to OAM each frame using the OAMDMA ($4014) register.
/// Writing N to this register causes the DMA circuitry inside the 2A03/07 to fully initialize the OAM by writing OAMDATA 256 times using successive bytes from starting at address $100*N). 
/// The CPU is suspended while the transfer is taking place.

pub struct Pallete {
    /// Address	Purpose
    /// $3F00	Universal background color
    /// $3F01-$3F03	Background palette 0
    /// $3F04	Normally unused color 1
    /// $3F05-$3F07	Background palette 1
    /// $3F08	Normally unused color 2
    /// $3F09-$3F0B	Background palette 2
    /// $3F0C	Normally unused color 3
    /// $3F0D-$3F0F	Background palette 3
    /// $3F10	Mirror of universal background color
    /// $3F11-$3F13	Sprite palette 0
    /// $3F14	Mirror of unused color 1
    /// $3F15-$3F17	Sprite palette 1
    /// $3F18	Mirror of unused color 2
    /// $3F19-$3F1B	Sprite palette 2
    /// $3F1C	Mirror of unused color 3
    /// $3F1D-$3F1F	Sprite palette 3
}

impl Pallete {
    
}

pub struct OAM {
    // We must handle 4 bytes at a time when working with this DRAM
    sprite_info: [u8; 256]
}

impl OAM {
    pub fn new() -> Self {
        Self {
            sprite_info: [0; 256]
        }
    }
}

pub struct PPU {
    vram: VRAM,
    oam: OAM,
    
    // internal registers
    v: u16, // During rendering, used for the scroll position. Outside of rendering, used as the current VRAM address.
    t: u16, // During rendering, specifies the starting coarse-x scroll for the next scanline and the starting y scroll for the screen. Outside of rendering, holds the scroll or VRAM address before transferring it to v.
    x: u16, // The fine-x position of the current scroll, used during rendering alongside v.
    w: bool, // Toggles on each write to either PPUSCROLL or PPUADDR, indicating whether this is the first or second write. Clears on reads of PPUSTATUS. Sometimes called the 'write latch' or 'write toggle'.

    increment: u8, // how much to increment the vram by per read/write
    sprite_pattern_address: u16,
    bg_pattern_address: u16,
    sprite_size: bool,
    mode: bool,
    generate_nmi: bool,
    master_slave_select: bool,
    num_sprites: usize,
    is_vblank: bool,
    sprite_hit: bool,

    base_nametable_address: u16,
    read_buffer: u8,
    oam_address: u8,
    x_scroll: u8,
    y_scroll: u8,

    is_greyscale: bool,
    clip_background: bool,
    clip_sprites: bool,
    show_background: bool,
    show_sprites: bool,
    emphasize_red: bool,
    emphasize_green: bool,
    emphasize_blue: bool,
}

// TODO: Reading any PPU port, including write-only ports $2000, $2001, $2003, $2005, $2006, returns the PPU I/O bus's value

fn get_bit(num: usize, i: u8) -> u8 {
    ((num >> i) & 1) as u8
}


fn set_bit(num: usize, idx: u8) -> u8 {
    (num | (1 << idx)) as u8
}


// fn set_n_bits(num: usize, idx: u8, n: u8) -> u8 {
//     unimplemented!()
// }

impl PPU {
    pub fn new() -> Self {
        Self {
            vram: VRAM::new(),
            oam: OAM::new(),
            oam_address: 0,

            v: 0,
            t: 0,
            x: 0,
            w: false, // false: 1st write, true: 2nd write

            increment: 1,
            sprite_pattern_address: 0x0000,
            bg_pattern_address: 0x0000,
            sprite_size: false,
            mode: false,
            master_slave_select: false,
            generate_nmi: false,
            num_sprites: 0,
            is_vblank: false,
            sprite_hit: false,

            base_nametable_address: 0x2000,
            read_buffer: 0,
            x_scroll: 0,
            y_scroll: 0,

            is_greyscale: false,
            clip_background: false,
            clip_sprites: false,
            show_background: false,
            show_sprites: false,
            emphasize_red: false,
            emphasize_green: false,
            emphasize_blue: false,
        }
    }

    /// $2000
    pub fn ppu_ctrl(&mut self, value: u8) {
        self.base_nametable_address = match value & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => 0x0000, // will never hit
        };
        self.increment = if get_bit(value.into(), 2) == 1 { 1 } else { 32 };
        self.sprite_pattern_address = if get_bit(value.into(), 3) == 1 { 0x1000 } else { 0x0000 };
        self.bg_pattern_address = if get_bit(value.into(), 4) == 1 { 0x0000 } else { 0x1000 };
        self.mode = get_bit(value.into(), 5) == 1; // 0 for 8x8, 1 for 8x16
        self.master_slave_select = get_bit(value.into(), 6) == 1; // (0: read backdrop from EXT pins; 1: output color on EXT pins)
        self.generate_nmi = get_bit(value.into(), 7) == 1; // Generate an NMI at the start of the vertical blanking interval (0: off; 1: on)

    }

    /// $2001
    pub fn ppu_mask(&mut self, value: u8) {
        self.is_greyscale = get_bit(value.into(), 0) == 1;
        self.clip_background = get_bit(value.into(), 1) == 1;
        self.clip_sprites = get_bit(value.into(), 2) == 1;
        self.show_background = get_bit(value.into(), 3) == 1;
        self.show_sprites = get_bit(value.into(), 4) == 1;
        self.emphasize_red = get_bit(value.into(), 5) == 1;
        self.emphasize_green = get_bit(value.into(), 6) == 1;
        self.emphasize_blue = get_bit(value.into(), 7) == 1;
    }

    /// $2002
    pub fn ppu_status(&mut self) -> u8 {
        // 7  bit  0
        // ---- ----
        // VSO. ....
        // |||| ||||
        // |||+-++++- PPU open bus. Returns stale PPU bus contents.
        // ||+------- Sprite overflow. The intent was for this flag to be set
        // ||         whenever more than eight sprites appear on a scanline, but a
        // ||         hardware bug causes the actual behavior to be more complicated
        // ||         and generate false positives as well as false negatives; see
        // ||         PPU sprite evaluation. This flag is set during sprite
        // ||         evaluation and cleared at dot 1 (the second dot) of the
        // ||         pre-render line.
        // |+-------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
        // |          a nonzero background pixel; cleared at dot 1 of the pre-render
        // |          line.  Used for raster timing.
        // +--------- Vertical blank has started (0: not in vblank; 1: in vblank).
        //         Set at dot 1 of line 241 (the line *after* the post-render
        //         line); cleared after reading $2002 and at dot 1 of the
        //         pre-render line.

        // TODO(backlog): setup working PPU open bus
        // clear write latch
        self.w = false;

        let mut val = 0b0000_0000;

        if self.num_sprites > 8 {
            val = set_bit(val.into(), 5);
        }

        if self.sprite_hit {
            val = set_bit(val.into(), 6);
        }

        if self.is_vblank {
            val = set_bit(val.into(), 7);
        }
        val
        
    }

    /// $2003
    pub fn oam_addr(&mut self, value: u8) {
        // Write the address of OAM you want to access here. 
        // Most games just write $00 here and then use OAMDMA.
        self.oam_address = value;
    }

    /// $2004
    pub fn oam_data_read(&mut self) -> u8 {
        self.oam.sprite_info[self.oam_address as usize]
    }

    /// $2004
    pub fn oam_data_write(&mut self, value: u8) {
        // Should we ignore writes because DMA is usually always used over this? 
        // Wiki says partial writes can cause corruption
        self.oam.sprite_info[self.oam_address as usize] = value;
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    /// $2005
    pub fn ppu_scroll(&mut self, value: u8) {
        if self.w == false {
            self.x_scroll = value;
            self.w = true;
        } else {
            self.y_scroll = value;
            self.w = false;
        }
    }

    /// $2006
    pub fn ppu_addr(&mut self, value: u8) {
        if !self.w {
            // update low byte of t
            self.t = value as u16;
            self.w = true;
        } else {
            // update high byte of t
            self.t |= (value as u16) << 8;
            self.v = self.t;
        }
    }

    // $2007
    pub fn ppu_data_read(&mut self) -> u8 {
        let old_buffer = self.read_buffer;

        let read_result = self.vram.get(self.v.into());
        self.read_buffer = read_result;

        // increment v by bit 2 of $2000 of VRAM
        self.v = self.v.wrapping_add(((self.vram.get(0x2000) & 0b10) >> 1) as u16);

        old_buffer
    }

    /// $2007
    pub fn ppu_data_write(&mut self, value: u8) {
        self.vram.set(self.v.into(), value);

        // increment v by bit 2 of $2000 of VRAM
        self.v = self.v.wrapping_add(((self.vram.get(0x2000) & 0b10) >> 1) as u16);
    }

    /// $4014
    pub fn oam_dma(&mut self, mem_slice: &[u8]) {
        self.oam.sprite_info = mem_slice.try_into().unwrap();
    }
}