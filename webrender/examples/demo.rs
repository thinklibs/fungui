#[macro_use]
extern crate stylish;
extern crate stylish_webrender;
extern crate glutin;
extern crate image;

use glutin::{WindowEvent as WiEvent, Event,
             VirtualKeyCode as KeyC, GlRequest,
             Api, ElementState as ElSt, MouseButton as MsBtn, GlContext,
             KeyboardInput, MouseScrollDelta};

use std::thread;
use std::time::Duration;
use std::env;
use std::fs;

const TARGET_FPS: u32 = 60;

struct TestLoader;

impl stylish_webrender::Assets for TestLoader {
    fn load_font(&self, name: &str) -> Option<Vec<u8>> {
        use std::io::Read;
        let mut file = if let Ok(f) = fs::File::open(format!("res/{}.ttf", name)) {
            f
        } else { return None; };
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        Some(data)
    }
    fn load_image(&self, name: &str) -> Option<stylish_webrender::Image> {
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
                    components: stylish_webrender::Components::BGRA,
                    data: {
                        let mut data = img.into_raw();
                        for d in data.chunks_mut(4) {
                            let a = d[3] as u32;
                            let r = ((d[0] as u32 * a) / 255) as u8;
                            d[0] = ((d[2] as u32 * a) / 255) as u8;
                            d[1] = ((d[1] as u32 * a) / 255) as u8;
                            d[2] = r;
                        }
                        data
                    },
                    is_opaque: false,
                })
            },
            _ => {
                let img = img.to_rgb();
                Some(stylish_webrender::Image {
                    width: img.width(),
                    height: img.height(),
                    components: stylish_webrender::Components::RGB,
                    data: img.into_raw(),
                    is_opaque: true,
                })
            },
        }
    }
}

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("Stylish")
        .with_dimensions(800, 480);
    let context_b = glutin::ContextBuilder::new()
        .with_stencil_buffer(8)
        .with_depth_buffer(24)
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 2)))
        .with_gl_profile(glutin::GlProfile::Core);
    let gl_window = glutin::GlWindow::new(window, context_b, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
    }

    let mut manager = stylish::Manager::new();

    if fs::metadata("./res").ok().map_or(true, |v| !v.is_dir()) {
        env::set_current_dir("../").unwrap();
    }

    let mut renderer = stylish_webrender::WebRenderer::new(
        |n| gl_window.get_proc_address(n),
        TestLoader,
        &mut manager,
    )
        .unwrap();

    manager.add_node_str(r##"
background
"##).unwrap();
    manager.add_node_str(r##"
top_bar {
    rust_logo
    "Stylish Demo"
}
"##).unwrap();
    manager.add_node_str(r##"
text_box {
    cbox(w=20,h=16,col="#FF0000")
    " Hello world this needs to be long enough to overflow "
    cbox(w=10,h=24,col="#00FF00")
    " and have a mix of text, elements and "
    rust_logo
    " images. Formatting like "
    "bold"(bold=true)
    " and colors: "
    "A"(color="#ff0000")
    "l"(color="#ff3500")
    "s"(color="#ff6a00")
    "o"(color="#ff9e00")
    " "(color="#ffd300")
    "s"(color="#f6ff00")
    "u"(color="#c1ff00")
    "p"(color="#8dff00")
    "p"(color="#58ff00")
    "o"(color="#23ff00")
    "r"(color="#00ff12")
    "t"(color="#00ff46")
    "s"(color="#00ff7b")
    " "(color="#00ffb0")
    "l"(color="#00ffe5")
    "o"(color="#00e5ff")
    "t"(color="#00b0ff")
    "s"(color="#007bff")
    " "(color="#0046ff")
    "o"(color="#0012ff")
    "f"(color="#2300ff")
    " "(color="#5800ff")
    "c"(color="#8d00ff")
    "o"(color="#c100ff")
    "l"(color="#f600ff")
    "o"(color="#ff00d3")
    "r"(color="#ff009e")
    "s"(color="#ff006a")
    "! "(color="#ff0035")
    cbox(w=70,h=24,col="#FF00FF")
}
"##).unwrap();
    manager.add_node_str(r##"
grid_box {
    text_box {
        "Grid layouts"
    }
    gradient(a="#83a4d4", b="#b6fbff")
    text_box {
        "work as well!"
    }
    gradient(a="#C02425", b="#F0CB35")
    text_box {
        "somewhat"
    }
    gradient(a="#C02425", b="#F0CB35")
}
"##).unwrap();
    manager.add_node_str(r##"
scroll_box {
    inner {
        "Example text
A B C"
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Quisque pharetra arcu vel urna tincidunt consectetur. Vivamus non nibh at mauris consectetur egestas. Ut pellentesque lorem et elit venenatis, nec rutrum lorem interdum. Ut pellentesque velit sed leo pulvinar blandit. Donec et eros posuere, ultrices tellus nec, dapibus ex. In elementum ligula vel tristique rutrum. Fusce id quam massa. Pellentesque in lectus eu felis venenatis gravida. Donec et velit vel turpis auctor imperdiet. Fusce eleifend sit amet lacus a placerat. Sed suscipit nisi nec nulla pretium accumsan. Curabitur aliquet sed magna id lobortis."

        "Vestibulum cursus nulla a sollicitudin semper. Duis eu est malesuada orci placerat porttitor vel fringilla libero. Mauris vitae nulla quis turpis vestibulum tincidunt eget egestas tortor. Quisque tincidunt eleifend nunc viverra venenatis. Phasellus ullamcorper libero id enim volutpat malesuada quis in sem. Fusce augue nibh, aliquam congue nibh quis, bibendum tempor libero. Mauris id lectus ac justo imperdiet accumsan sit amet eget eros. Cras orci odio, facilisis ut mollis id, faucibus ac lectus. Integer diam nibh, lacinia sit amet velit non, mattis ullamcorper elit."

        "Curabitur nec tortor at arcu feugiat fermentum id quis ex. Pellentesque leo erat, tempus vitae auctor luctus, auctor non dui. In hac habitasse platea dictumst. Cras posuere ullamcorper viverra. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. In lobortis mauris enim, ut bibendum lorem tincidunt eget. Etiam faucibus mollis tincidunt. Cras efficitur vestibulum pretium. Nunc elementum tellus at laoreet interdum. Quisque placerat, arcu sed pulvinar finibus, orci ante pretium tellus, sit amet ultricies risus odio non massa. Praesent fermentum velit eu tortor egestas, id convallis lectus cursus. Nam non fringilla nunc. Fusce a laoreet enim, non gravida diam. Curabitur malesuada, orci ac pellentesque consectetur, augue lacus tincidunt nunc, sit amet posuere massa turpis non lacus. Vivamus pretium interdum suscipit. Vivamus bibendum interdum dolor."
    }
}
"##).unwrap();
    // Macro demo
    manager.add_node(node!{
        dragable(x=200, y=60) {
            @text("Drag me!")
        }
    });
    manager.load_styles("base", r##"
dragable(x=x, y=y) {
    x = x,
    y = y,
    background_color = "#FF00FF",
    layout = "lined",
    max_width = 200,
    min_width = 16,
    height = 16,

    filters = filters(
        "hue_rotate", (x / parent_width) * 3.14,
        "brightness", 1.0 - (y / parent_height)),
}
dragable > @text {
    font = "font/FiraSans-Regular",
    font_size = 12,
    font_color = rgb(0, 0, 0),
}

grid_box {
    layout = "grid",
    x = 16,
    y = 200,
    width = (parent_width / 2) - 32,
    height = parent_height - 216,
    shadow = shadow(0.0, 0.0, rgba(0, 0, 0, 1.0), 8.0, 0.0, "inset"),
    layout = "grid",
    columns = 3,
    rows = 2,
    margin = 16,
    spacing = 16,
    force_size = true,
}

scroll_box {
    x = (parent_width / 2) + 16,
    y = 200,
    width = (parent_width / 2) - 32,
    height = parent_height - 216,
    background_color = "#F49E42",
    shadow = shadows(
        shadow(4.0, 4.0, rgba(0, 0, 0, 0.28), 8.0, 0.0, "outset"),
        shadow(0.0, 0.0, rgba(0, 0, 0, 0.14), 4.0, 0.0, "outset")),
    border_width = border_width(27.0),
    border = border_image("border", 27, 27, "stretch"),
}

scroll_box > inner {
    x = 30,
    y = 30,
    width = parent_width - (30 * 2),
    height = parent_height - (30 * 2),
    layout = "lined",
    clip_overflow = true,
    can_scroll = true,
}

inner(scroll_y=scroll_y) {
    scroll_y = scroll_y,
}

inner > @text {
    font = "font/FiraSans-Regular",
    font_size = 17,
    font_color = rgb(0, 0, 0),
}

gradient(a=a, b=b) {
    background_color = gradient(deg(-90.0),
        stop(0.0, a),
        stop(1.0, b)),
}

gradient(a=a, b=b, hover=true) {
    background_color = gradient(deg(-90.0),
        stop(0.0, b),
        stop(1.0, a)),
}

background {
    x = 0,
    y = 0,
    width = parent_width,
    height = parent_height,
    background_color = "#EEEEEE",
}
root(width=width, height=height) > top_bar {
    x = 0,
    y = 0,
    width = width,
    height = 56,
    background_color = "#F49E42",
    shadow = shadows(
        shadow(0.0, 4.0, rgba(0, 0, 0, 0.28), 8.0, 0.0, "outset"),
        shadow(0.0, 0.0, rgba(0, 0, 0, 0.14), 4.0, 0.0, "outset")),
}

top_bar > rust_logo {
    x = 16,
    y = 0,
    width = 56,
    height = 56,
    image = "rust-logo-64",
}

top_bar > @text {
    x = 16 + 56 + 8,
    y = 16,
    width = 200,
    height = 24,
    font = "font/FiraSans-Regular",
    font_size = 24,
    font_color = rgb(0, 0, 0),
}

text_box {
    x = 16,
    y = 100,
    max_width = parent_width - 32,
}

text_box {
    background_color = rgba(0, 0, 0, 0.3),
    layout = "lined",
    line_height = 17,
    auto_size = true,
    shadow = shadow(4.0, 4.0, rgba(0, 0, 0, 0.5), 8.0, 0.0, "outset"),
    clip_overflow = true,
}
text_box > @text {
    font = "font/FiraSans-Regular",
    font_size = 17,
    font_color = rgb(0, 0, 0),
}
text_box > @text(bold=true) {
    font = "font/FiraSans-Bold",
}
text_box > @text(color=color) {
    font_color = color,
}
grid_box > text_box {
    border_width = border_width(8.0, 8.0, 8.0, 8.0),
    border = border(
        bside("#444444", "none"),
        bside("#444444", "none"),
        bside("#444444", "none"),
        bside("#444444", "inset")),
}

text_box > rust_logo {
    width = 17,
    height = 17,
    image = "rust-logo-32",
}

cbox(w=width, h=height, col=color) {
    width = width,
    height = height,
    background_color = color,
}

"##).unwrap();

    let target_frame_time = Duration::from_secs(1) / TARGET_FPS;

    let mut last_hover: Option<stylish::Node<_>> = None;

    struct Drag {
        target: stylish::Node<stylish_webrender::Info>,
        x: i32,
        y: i32,
    }
    let mut current_drag: Option<Drag> = None;
    let mut mouse_pos = (0, 0);

    'main_loop:
    loop {
        let mut finished = false;
        let start = ::std::time::Instant::now();

        let (width, height) = gl_window.get_inner_size_pixels().unwrap();
        renderer.layout(&mut manager, width, height);

        events_loop.poll_events(|event| {
            if let Event::WindowEvent {event, .. } = event {
            match event {
                WiEvent::Closed |
                WiEvent::KeyboardInput {input: KeyboardInput {virtual_keycode: Some(KeyC::Escape), .. } , .. } => {
                    finished = true;
                    return;
                },
                WiEvent::MouseInput{state: ElSt::Pressed, button: MsBtn::Left, ..} => {
                    for n in manager.query_at(mouse_pos.0, mouse_pos.1)
                        .matches()
                    {
                        if n.get_property::<i32>("x").is_some() && n.get_property::<i32>("y").is_some() {
                            current_drag = Some(Drag {
                                target: n,
                                x: mouse_pos.0,
                                y: mouse_pos.1,
                            });
                            break;
                        }
                    }
                },
                WiEvent::MouseInput{state: ElSt::Released, button: MsBtn::Left, ..} => {
                    current_drag = None;
                },
                WiEvent::MouseMoved{position: (x, y), ..} => {
                    let (x, y) =  (x as i32, y as i32);
                    mouse_pos = (x, y);
                    if let Some(last_hover) = last_hover.take() {
                        last_hover.set_property("hover", false);
                    }
                    if let Some(drag) = current_drag.as_mut() {
                        let dx = x - drag.x;
                        let dy = y - drag.y;

                        let lx = drag.target.get_property::<i32>("x").unwrap();
                        let ly = drag.target.get_property::<i32>("y").unwrap();
                        drag.target.set_property("x", lx + dx);
                        drag.target.set_property("y", ly + dy);

                        drag.x = x;
                        drag.y = y;
                    } else if let Some(hover) = manager.query_at(x, y)
                        .name("gradient")
                        .matches()
                        .next()
                    {
                        hover.set_property("hover", true);
                        last_hover = Some(hover);
                    }
                },
                WiEvent::MouseWheel{delta, ..} => {
                    for node in manager.query_at(mouse_pos.0, mouse_pos.1)
                        .matches()
                    {
                        if node.get_value::<bool>("can_scroll").unwrap_or(false) {
                            let mut max = 0;
                            for n in node.children() {
                                let obj = n.render_object();
                                let m = obj.draw_rect.y + obj.draw_rect.height;
                                if m > max {
                                    max = m;
                                }
                            }
                            max -= node.render_object().draw_rect.height;
                            if max < 0 {
                                max = 0;
                            }
                            let oy = node.get_property::<f64>("scroll_y").unwrap_or(0.0);
                            let y = match delta {
                                MouseScrollDelta::LineDelta(_, y) => y,
                                MouseScrollDelta::PixelDelta(_, y) => y,
                            };
                            node.set_property("scroll_y", (oy + y as f64 * 5.0).min(0.0).max(-max as f64));
                            break;
                        }
                    }
                },
                _ => {}
            }
            }
        });

        if finished {
            break;
        }

        renderer.render(&mut manager, width, height);

        gl_window.swap_buffers().unwrap();

        let frame_time = start.elapsed();
        if frame_time < target_frame_time {
            thread::sleep(target_frame_time - frame_time);
        }

    }
}
