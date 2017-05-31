extern crate stylish;
extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{Canvas, BlendMode};
use sdl2::video::Window;
use sdl2::pixels::Color;

use std::thread;
use std::time::Duration;

struct GridLayout {
    count: usize,
    grid_size: i32,
    width: i32,
}

impl GridLayout {
    fn new(obj: &stylish::RenderObject) -> Box<stylish::LayoutEngine> {
        let size = obj.get_value::<i32>("grid_size").unwrap_or(1);
        Box::new(GridLayout {
            count: 0,
            grid_size: size,
            width: obj.draw_rect.width,
        })
    }
}

impl stylish::LayoutEngine for GridLayout {
    fn position_element(&mut self, _obj: &stylish::RenderObject) -> stylish::Rect {
        let pos = self.count as i32;
        self.count += 1;
        let w = self.width / self.grid_size;
        stylish::Rect {
            x: (pos % w) * self.grid_size,
            y: (pos / w) * self.grid_size,
            width: self.grid_size,
            height: self.grid_size,
        }
    }
}

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
    manager.add_layout_engine("grid", GridLayout::new);

    manager.add_node(stylish::Node::from_str(r##"
box(x=15, y=15, width=100, height=150) {
    sub(x=5, y=20, width=20, height=20)
    sub(x=25, y=20, width=20, height=20, color="#0000FF")
    sub(x=45, y=20, width=20, height=20, color="#FF00FF")

    grid(x=5, y=50, width=20, height = 20, size=10) {
        sub(color="#0000FF")
        sub(color="#00FF00")
        sub(color="#FF0000")
        sub(color="#FFFF00")
    }

    grid(x=35, y=50, width=25, height = 20, size=5) {
        sub(color="#0000FF")
        sub(color="#00FF00")
        sub(color="#FF0000")
        sub(color="#FFFF00")
        sub(color="#0000FF")
        sub(color="#00FF00")
        sub(color="#FF0000")
        sub(color="#FFFF00")
        sub(color="#0000FF")
        sub(color="#00FF00")
        sub(color="#FF0000")
        sub(color="#FFFF00")
        sub(color="#0000FF")
        sub(color="#00FF00")
        sub(color="#FF0000")
        sub(color="#FFFF00")
    }
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
    color = "#FFFFFF",
}
box > sub(x=x, y=y, width=width, height=height) {
    x = x,
    y = y,
    width = width,
    height = height,
    color = "#00ff00",
}

grid(x=x, y=y, width=width, height=height, size=size) {
    x = x,
    y = y,
    width = width,
    height = height,
    layout = "grid",
    grid_size = size,
}

sub(color=col) {
    color = col,
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

fn parse_color(v: &str) -> Option<(u8, u8, u8, u8)> {
    if v.chars().next() == Some('#') {
        let col = &v[1..];
        if col.len() == 6 || col.len() == 8 {
            Some((
                u8::from_str_radix(&col[..2], 16)
                    .unwrap(),
                u8::from_str_radix(&col[2..4], 16)
                    .unwrap(),
                u8::from_str_radix(&col[4..6], 16)
                    .unwrap(),
                if col.len() == 8 {
                    u8::from_str_radix(&col[6..8], 16)
                        .unwrap()
                } else { 255 },
            ))
        } else {
            None
        }
    } else {
        None
    }
}

struct CanvasRenderer<'a> {
    canvas: &'a mut Canvas<Window>,
}

impl <'a> stylish::RenderVisitor for CanvasRenderer<'a> {
    fn visit(&mut self, obj: &stylish::RenderObject) {
        use sdl2::rect::Rect;
        let color = obj.get_value::<String>("color")
            .and_then(|v| parse_color(&v))
            .unwrap_or((255, 255, 255, 0));
        self.canvas.set_draw_color(Color::RGBA(
            color.0,
            color.1,
            color.2,
            color.3,
        ));
        self.canvas.fill_rect(Rect::new(
            obj.draw_rect.x, obj.draw_rect.y,
            obj.draw_rect.width as u32, obj.draw_rect.height as u32,
        )).unwrap();
    }
}