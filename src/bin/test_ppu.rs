use std::cell::RefCell;
use std::process;
use std::rc::Rc;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;
use sdl2::Sdl;

static BLACK: Color = Color::RGB(0, 0, 0);
static WHITE: Color = Color::RGB(255, 255, 255);

pub fn main_loop(
    ctx: Rc<RefCell<Sdl>>,
    rect: Rc<RefCell<Rect>>,
    canvas: Rc<RefCell<WindowCanvas>>,
) -> impl FnMut() {
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

        let _ = canvas.borrow_mut().set_draw_color(BLACK);
        let _ = canvas.borrow_mut().clear();
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
        .window("Nemsys", 640, 480)
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
