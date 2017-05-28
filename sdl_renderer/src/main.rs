extern crate stylish;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{Canvas, BlendMode};
use sdl2::video::Window;
use sdl2::pixels::Color;

use std::thread;
use std::time::Duration;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("SDL2", 800, 480)
        .build()
        .unwrap();

    let mut canvas = window.into_canvas()
        .accelerated()
        .build()
        .unwrap();

    let mut event_pump = sdl_context.event_pump()
        .unwrap();

    let mut manager = stylish::Manager::new();
    manager.add_node(stylish::Node::from_str(r##"
box(x=15, y=15, width=100, height=150) {
    sub(x=5, y=20, width=20, height=20)
    sub(x=25, y=20, width=20, height=20, color="#0000FF")
    sub(x=45, y=20, width=20, height=20, color="#FF00FF")
}
"##).unwrap());
    manager.load_styles("base", r##"
root {
    width = 0,
    height = 0,
}
box(x=x, y=y, width=width, height=height) {
    x = x,
    y = y,
    width = (800 - x) - 15,
    height = height,
    color = "#ff0000",
}
box > sub(x=x, y=y, width=width, height=height) {
    x = x,
    y = y,
    width = width,
    height = height,
    color = "#00ff00",
}

box > sub(color=col) {
    color = col + "AA",
}
"##).unwrap();

    let b = manager.query()
        .name("box")
        .matches()
        .next()
        .unwrap();

    'main_loop:
    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
                    break 'main_loop;
                },
                Event::MouseMotion{x, y, ..} => {
                    b.set_property("x", x as i32);
                    b.set_property("y", y as i32);
                }
                _ => {}
            }
        }

        let (width, height) = canvas.logical_size();

        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.set_blend_mode(BlendMode::Blend);
        canvas.clear();
        {
            manager.render(&mut CanvasRenderer {
                canvas: &mut canvas,
            }, width as i32, height as i32);
        }

        canvas.present();

        thread::sleep(Duration::from_millis(15));
    }
}

struct CanvasRenderer<'a> {
    canvas: &'a mut Canvas<Window>,
}

impl <'a> stylish::RenderVisitor for CanvasRenderer<'a> {
    fn visit(&mut self, obj: &stylish::RenderObject) {
        use sdl2::rect::Rect;
        self.canvas.set_draw_color(Color::RGBA(
            obj.color.0,
            obj.color.1,
            obj.color.2,
            obj.color.3,
        ));
        self.canvas.fill_rect(Rect::new(
            obj.draw_rect.x, obj.draw_rect.y,
            obj.draw_rect.width as u32, obj.draw_rect.height as u32,
        )).unwrap();
    }
}