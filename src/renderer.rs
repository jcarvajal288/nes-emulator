extern crate sdl2; 

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::time::Duration;
use sdl2::rect::Point;

const SCREEN_WIDTH: u32 = 341;
const SCREEN_HEIGHT: u32 = 261;

//type Frame = [[Color; SCREEN_WIDTH]; SCREEN_HEIGHT];

pub struct Renderer {
	canvas: Canvas<Window>,
}

impl Renderer {

	pub fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
		self.canvas.set_draw_color(color);
		self.canvas.draw_point(Point::new(x,y))		
			.expect("ERROR: could not draw point");
	}

	pub fn show_frame(&mut self) {
		self.canvas.present();
		self.canvas.clear();
	}
}

pub fn create_renderer() -> Renderer {

	let sdl_content = sdl2::init().unwrap();
	let video_subsystem = sdl_content.video().unwrap();
	let window = video_subsystem.window("Yet Another NES Emulator", SCREEN_WIDTH, SCREEN_HEIGHT)
		.position_centered()
		.opengl()
		.build()
		.map_err(|e| e.to_string()).unwrap();
	let mut canvas = window
		.into_canvas()
		.build()
		.map_err(|e| e.to_string()).unwrap();
	canvas.set_draw_color(Color::BLACK);
	canvas.clear();
	canvas.present();

	return Renderer {
		canvas: canvas,
	}
}
 
/*
pub fn render() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
 
    let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
        .position_centered()
        .build()
        .unwrap();
 
    let mut canvas = window.into_canvas().build().unwrap();
 
    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        i = (i + 1) % 255;
        canvas.set_draw_color(Color::RGB(i, 64, 255 - i));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
*/