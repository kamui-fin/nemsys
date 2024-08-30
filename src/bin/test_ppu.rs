use std::cell::RefCell;
use std::rc::Rc;
use std::{default, process};

use log::{error, LevelFilter};
use nemsys::cpu::Cpu;
use nemsys::mappers::{Mapper, NROM};
use nemsys::ppu::{self, PPU};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormat};
use sdl2::rect::Rect;
use sdl2::render::{Texture, WindowCanvas};
use sdl2::Sdl;
use simplelog::{ColorChoice, CombinedLogger, Config, TermLogger, TerminalMode};
use std::thread::sleep;
use std::time::Duration;

#[cfg(target_family = "wasm")]
use nemsys::ppu::emscripten;

/*
- Set up the renderer with the SDL_RENDERER_ACCELERATED flag (and SDL_RENDERER_PRESENTVSYNC if you want it to pace itself automatically)
- Make an SDL_Texture of the desired size, with the appropriate SDL_PIXELFORMAT flag (in my case, my ARGB32 data needed BGRA32 format. wut) and the SDL_TEXTUREACCESS_STREAMING flag
- Right before it's time to modify your texture, use SDL_LockTexture and point it to the texture you made, pass it a pointer to a plain pixel data array (essentially you want a pointer type variable such as `u32* pixels`, then you call it like `reinterpret_cast<void**>(&pixels)` in the arguments, and also define the texture pitch (number of bytes for a row of pixels in the texture)
- Modify the pixel data using the same pointer address at this point, based on your actual array(s) holding the data on the emulator's side
- When done, use sdl_unlocktexture, and the editing process is done.
- At the end of each frame, you'd use SDL_RenderCopy to put the texture (or whatever rect/surface you prefer tying it to) on the "queue" for SDL_RenderPresent to consume.
*/

static BLACK: Color = Color::RGB(0, 0, 0);
static WHITE: Color = Color::RGB(255, 255, 255);

static WIDTH: usize = 256;
static HEIGHT: usize = 240;

struct Display {
    pub width: u32,
    pub height: u32,
    pub ctx: Rc<RefCell<Sdl>>,
    pub sdl_canvas: sdl2::render::Canvas<sdl2::video::Window>,
    pub tex_creator: sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    pub texture: RefCell<Texture<'static>>,
    pub data: Rc<RefCell<Vec<u32>>>,
}

impl Display {
    fn new(width: u32, height: u32) -> Self {
        let ctx = sdl2::init().unwrap();
        let video_ctx = ctx.video().unwrap();

        let window = match video_ctx
            .window("Nemsys", width, height)
            .position_centered()
            .opengl()
            .build()
        {
            Ok(window) => window,
            Err(err) => panic!("failed to create window: {}", err),
        };

        let sdl_canvas = match window.into_canvas().present_vsync().build() {
            Ok(canvas) => canvas,
            Err(err) => panic!("failed to create canvas: {}", err),
        };
        let tex_creator = sdl_canvas.texture_creator();
        let texture = tex_creator
            .create_texture(
                sdl2::pixels::PixelFormatEnum::RGBA8888,
                sdl2::render::TextureAccess::Streaming,
                width as u32,
                height as u32,
            )
            .unwrap();

        let texture = unsafe { std::mem::transmute::<_, Texture<'static>>(texture) };
        let texture = RefCell::new(texture);

        let ctx = Rc::new(RefCell::new(ctx));

        let default_color = Color::RGB(255, 255, 255)
            .to_u32(&sdl2::pixels::PixelFormatEnum::RGBA8888.try_into().unwrap());

        Self {
            width,
            height,
            ctx,
            sdl_canvas,
            texture,
            tex_creator,
            data: Rc::new(RefCell::new(vec![default_color; (width * height) as usize])),
        }
    }

    fn flush(&mut self) {
        let mut texture = self.texture.borrow_mut();
        texture
            .update(None, self.data_raw(), (self.width * 4) as usize)
            .unwrap();
        self.sdl_canvas.clear();
        self.sdl_canvas.copy(&texture, None, None).unwrap();
        self.sdl_canvas.present();
    }

    fn data_raw(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.data.borrow().as_ptr() as *const u8,
                self.data.borrow().len() * 4,
            )
        }
    }

    fn main_loop(&mut self) {
        let ppu = Rc::new(RefCell::new(PPU::new(Rc::clone(&self.data))));
        let mut cpu = Cpu::new(Rc::clone(&ppu));
        let rom = NROM::from_ines_rom(
            "donkey_kong.nes",
            &mut ppu.borrow_mut().vram,
            &mut cpu.memory,
        )
        .unwrap();

        cpu.init_pc();
        

        loop {
            let mut events = self.ctx.borrow_mut().event_pump().unwrap();
            for event in events.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => {
                        process::exit(1);
                    }
                    _ => {}
                }
            }

            if cpu.num_cycles > 100000 {
                // error!("{:#?}", &ppu.borrow().vram.buffer[0x2100..0x2200]);
                // panic!();
            }

            cpu.tick((341 / 3) as usize); // runs cpu for equivalent num_cycles
            ppu.borrow_mut().tick(); // runs ppu for 1 scanline

            if ppu.borrow().is_vblank {
                self.flush();

                if ppu.borrow().generate_nmi {
                    cpu.generate_nmi();
                }
            }
        }
    }

    pub fn display_pattern_table(&mut self, ppu: Rc<RefCell<PPU>>) {
        let palette = [
            BLACK,
            Color::RGB(219, 1, 84),
            Color::RGB(82, 221, 78),
            Color::RGB(143, 225, 237),
        ];
        let pixsize: usize = 3;
        let tile_size: usize = pixsize * 8;
        let mut last_tile_pos = 0x1000;
        for k in 0..256 {
            let tile = &ppu.borrow().vram.buffer[last_tile_pos..(last_tile_pos + 16)];
            for r in 0..8 {
                for c in 0..8 {
                    let first_bit = (tile[r].reverse_bits() >> c) & 1;
                    let second_bit = (tile[r + 8].reverse_bits() >> c) & 1;
                    let color_index = (second_bit << 1) | first_bit;
                    let color = palette[color_index as usize];

                    // draw pixel with color at given square coordinates
                    self.sdl_canvas.set_draw_color(color);
                    let x_offset = (k * tile_size) % self.width as usize;
                    let y_offset = ((k * tile_size) / self.width as usize) * tile_size;
                    let x = r * pixsize + x_offset; // X-coordinate
                    let y = c * pixsize + y_offset; // Y-coordinate
                    self.sdl_canvas
                        .fill_rect(Rect::new(
                            y as i32,
                            x as i32,
                            pixsize as u32,
                            pixsize as u32,
                        ))
                        .unwrap();
                }
            }
            last_tile_pos = last_tile_pos + 16;
        }

        self.sdl_canvas.present();
    }
}

fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Off,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();
    let mut canvas = Display::new(256, 240);

    // #[cfg(target_family = "wasm")]
    // emscripten::set_main_loop_callback(canvas.main_loop());

    #[cfg(not(target_family = "wasm"))]
    {
        canvas.main_loop();
    }
}
