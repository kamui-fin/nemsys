use std::cell::RefCell;
use std::process;
use std::rc::Rc;

use nemsys::cpu::Cpu;
use nemsys::mappers::{Mapper, NROM};
use nemsys::ppu;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormat};
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::Sdl;

static BLACK: Color = Color::RGB(0, 0, 0);
static WHITE: Color = Color::RGB(255, 255, 255);

static WIDTH: usize = 5 * 8 * 32;
static HEIGHT: usize = 5 * 8 * 16;

pub fn main_loop(
    ctx: Rc<RefCell<Sdl>>,
    rect: Rc<RefCell<Rect>>,
    canvas: Rc<RefCell<WindowCanvas>>,
) -> impl FnMut() {
    let mut vram = ppu::memory::VRAM::new();
    let mut cpu = Cpu::new(&mut vram);
    NROM::load_ines_rom("donkey_kong.nes", &mut vram, &mut cpu.memory).unwrap();

    let mut events = ctx.borrow_mut().event_pump().unwrap();

    move || {
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

        let palette = [
            BLACK,
            Color::RGB(219, 1, 84),
            Color::RGB(82, 221, 78),
            Color::RGB(143, 225, 237),
        ];
        let pixsize = 5;
        let mut last_tile_pos = 0x0000;

        // let grid_i = 0;
        // let grid_j = 0;

        let tile_size = pixsize * 8;

        for k in 0..512 {
            let tile = &vram.buffer[last_tile_pos..(last_tile_pos + 16)];
            for i in 0..8 {
                for j in 0..8 {
                    let first_bit = (tile[i].reverse_bits() >> j) & 1;
                    let second_bit = (tile[i + 8].reverse_bits() >> j) & 1;
                    let color_index = (second_bit << 1) | first_bit;
                    let color = palette[color_index as usize];

                    // draw pixel with color at given square coordinates
                    canvas.borrow_mut().set_draw_color(color);
                    let x_offset = (k * tile_size) % WIDTH;
                    let y_offset = ((k * tile_size) / WIDTH) * tile_size;
                    let x = i * pixsize + x_offset; // X-coordinate
                    let y = j * pixsize + y_offset; // Y-coordinate
                    canvas
                        .borrow_mut()
                        .fill_rect(Rect::new(
                            x as i32,
                            y as i32,
                            pixsize as u32,
                            pixsize as u32,
                        ))
                        .unwrap();
                }
            }
            last_tile_pos = last_tile_pos + 16;
        }
        let _ = canvas.borrow_mut().present();
    }
}

// Resources
//     https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_Wasm
//     https://puddleofcode.com/story/definitive-guide-to-rust-sdl2-and-emscriptem/

// To build locally:
//     cargo run

// To build for the web:
//     rustup target add wasm32-unknown-emscripten
//     export EMCC_CFLAGS="-s USE_SDL=2"
//     cargo build --target wasm32-unknown-emscripten && open index.html
fn main() {
    let ctx = sdl2::init().unwrap();
    let video_ctx = ctx.video().unwrap();

    let window = match video_ctx
        .window("Nemsys", WIDTH as u32, HEIGHT as u32)
        .position_centered()
        .opengl()
        .build()
    {
        Ok(window) => window,
        Err(err) => panic!("failed to create window: {}", err),
    };

    let canvas = match window.into_canvas().present_vsync().build() {
        Ok(canvas) => canvas,
        Err(err) => panic!("failed to create canvas: {}", err),
    };

    let rect = Rect::new(0, 0, 10, 10);

    let ctx = Rc::new(RefCell::new(ctx));
    let rect = Rc::new(RefCell::new(rect));
    let canvas = Rc::new(RefCell::new(canvas));

    #[cfg(target_family = "wasm")]
    use nemsys::ppu::emscripten;

    #[cfg(target_family = "wasm")]
    emscripten::set_main_loop_callback(main_loop(
        Rc::clone(&ctx),
        Rc::clone(&rect),
        Rc::clone(&canvas),
    ));

    #[cfg(not(target_family = "wasm"))]
    {
        use std::thread::sleep;
        use std::time::Duration;
        loop {
            main_loop(Rc::clone(&ctx), Rc::clone(&rect), Rc::clone(&canvas))();
            sleep(Duration::from_millis(10))
        }
    }
}
