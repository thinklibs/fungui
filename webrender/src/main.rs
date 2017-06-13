extern crate stylish;
extern crate stylish_webrender;
extern crate sdl2;
extern crate webrender;
extern crate webrender_traits;
extern crate gleam;
extern crate image;

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
    fn position_element(&mut self, _obj: &mut stylish::RenderObject<T>) -> stylish::Rect {
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

struct TestLoader;

impl stylish_webrender::Assets for TestLoader {
    fn load_font(&mut self, name: &str) -> Option<Vec<u8>> {
        use std::fs;
        use std::io::Read;
        let mut file = if let Ok(f) = fs::File::open(format!("res/{}.ttf", name)) {
            f
        } else { return None; };
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        Some(data)
    }
    fn load_image(&mut self, name: &str) -> Option<stylish_webrender::Image> {
        use std::fs;
        use std::io::BufReader;
        let file = BufReader::new(if let Ok(f) = fs::File::open(format!("res/{}.png", name)) {
            f
        } else { return None; });
        let img = if let Ok(val) = image::load(file, image::ImageFormat::PNG) {
            val
        } else {
            return None;
        };
        match img.color() {
            image::ColorType::RGBA(..) | image::ColorType::GrayA(..) => {
                let img = img.to_rgba();
                Some(stylish_webrender::Image {
                    width: img.width(),
                    height: img.height(),
                    components: stylish_webrender::Components::RGBA,
                    data: {
                        let mut data = img.into_raw();
                        for d in data.chunks_mut(4) {
                            let a = d[3] as u32;
                            d[0] = ((d[0] as u32 * a) / 255) as u8;
                            d[1] = ((d[1] as u32 * a) / 255) as u8;
                            d[2] = ((d[2] as u32 * a) / 255) as u8;
                        }
                        data
                    },
                })
            },
            _ => {
                let img = img.to_rgb();
                Some(stylish_webrender::Image {
                    width: img.width(),
                    height: img.height(),
                    components: stylish_webrender::Components::RGB,
                    data: img.into_raw(),
                })
            },
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

    let window = video_subsystem.window("Stylish", 800, 480)
        .opengl()
        .resizable()
        .build()
        .unwrap();

    let context = window.gl_create_context().unwrap();
    window.gl_make_current(&context).unwrap();

    let mut manager = stylish::Manager::new();
    manager.add_layout_engine("grid", GridLayout::new);

    let mut renderer = stylish_webrender::WebRenderer::new(
        |n| video_subsystem.gl_get_proc_address(n),
        TestLoader,
        &mut manager,
    )
        .unwrap();

    let mut event_pump = sdl_context.event_pump()
        .unwrap();

    manager.add_node_str(r##"
top_bar {
    menu
    "Inbox"
    search {
        icon
        "Search"
    }
}
"##).unwrap();
    manager.load_styles("base", r##"
root(width=width, height=height) > top_bar {
    x = 0,
    y = 0,
    width = width,
    height = 56,
    background_color = "#4285f4",
    shadow = shadows(
        shadow(0.0, 4.0, rgba(0, 0, 0, 0.28), 8.0, 0.0, "outset"),
        shadow(0.0, 0.0, rgba(0, 0, 0, 0.14), 4.0, 0.0, "outset")),
}
top_bar > menu {
    x = 24,
    y = 16,
    width = 24,
    height = 24,
    image = "menu_white",
}
top_bar > @text {
    x = 67,
    y = 35,
    width = 60,
    height = 24,
    font = "arial",
    font_size = 20,
    font_color = rgb(255, 255, 255),
}
root(width=width, height=height) > top_bar > search {
    width = width - 300,
    height = 36,
    x = 150,
    y = 10,
    background_color = rgba(255,255,255,0.15),
}
search > icon {
    x = 24,
    y = 6,
    width = 24,
    height = 24,
    image = "search_white",
}
search > @text {
    x = 67,
    y = 24,
    width = 60,
    height = 24,
    font = "arial",
    font_size = 17,
    font_color = rgb(255, 255, 255),
}
"##).unwrap();

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