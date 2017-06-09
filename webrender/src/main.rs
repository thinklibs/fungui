extern crate stylish;
extern crate stylish_webrender;
extern crate sdl2;
extern crate webrender;
extern crate webrender_traits;
extern crate gleam;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use std::thread;
use std::time::Duration;

const TARGET_FPS: u32 = 60;

struct GridLayout {
    count: usize,
    grid_size: i32,
    width: i32,
}

impl GridLayout {
    fn new<T>(obj: &stylish::RenderObject<T>) -> Box<stylish::LayoutEngine<T>> {
        let size = obj.get_value::<i32>("grid_size").unwrap_or(1);
        Box::new(GridLayout {
            count: 0,
            grid_size: size,
            width: obj.draw_rect.width,
        })
    }
}

impl <T> stylish::LayoutEngine<T> for GridLayout {
    fn position_element(&mut self, _obj: &stylish::RenderObject<T>) -> stylish::Rect {
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

    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_stencil_size(8);
    gl_attr.set_depth_size(24);
    gl_attr.set_context_major_version(3);
    gl_attr.set_context_minor_version(2);
    gl_attr.set_context_profile(sdl2::video::GLProfile::Core);

    let window = video_subsystem.window("SDL2", 800, 480)
        .opengl()
        .resizable()
        .build()
        .unwrap();

    let context = window.gl_create_context().unwrap();
    window.gl_make_current(&context).unwrap();

    let mut renderer = stylish_webrender::WebRenderer::new(|n| video_subsystem.gl_get_proc_address(n))
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
    width = width,
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

    let target_frame_time = Duration::from_secs(1) / TARGET_FPS;

    'main_loop:
    loop {
        let start = ::std::time::Instant::now();

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

        let (width, height) = window.drawable_size();
        renderer.render(&mut manager, width, height);

        window.gl_swap_window();

        let frame_time = start.elapsed();
        if frame_time < target_frame_time {
            thread::sleep(target_frame_time - frame_time);
        }

    }
}