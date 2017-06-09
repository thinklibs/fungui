
extern crate webrender;
extern crate webrender_traits;
extern crate gleam;
extern crate stylish;

use webrender::*;
use webrender_traits::*;
use std::error::Error;

// TODO: Don't box errors, error chain would be better
type WResult<T> = Result<T, Box<Error>>;

pub struct WebRenderer {
    renderer: Renderer,
    api: RenderApi,
    frame_id: Epoch,
}

impl WebRenderer {
    pub fn new<F>(load_fn: F) -> WResult<WebRenderer>
        where F: Fn(&str) -> *const ()
    {
        let gl = unsafe {gleam::gl::GlFns::load_with(|f|
            load_fn(f) as *const _
        )};
        let options = webrender::RendererOptions {
            device_pixel_ratio: 1.0,
            resource_override_path: None,
            debug: false,
            clear_framebuffer: true, // TODO: Remove
            .. Default::default()
        };
        let (renderer, sender) = webrender::Renderer::new(gl, options, webrender_traits::DeviceUintSize::new(800, 480)).unwrap();
        let api = sender.create_api();
        renderer.set_render_notifier(Box::new(Dummy));

        let pipeline = webrender_traits::PipelineId(0, 0);
        api.set_root_pipeline(pipeline);

        Ok(WebRenderer {
            renderer: renderer,
            api: api,
            frame_id: Epoch(0),
        })
    }

    pub fn render(&mut self, manager: &mut stylish::Manager<Info>, width: u32, height: u32) {
        self.frame_id.0 += 1;
        let pipeline = webrender_traits::PipelineId(0, 0);
        self.renderer.update();
        let size = webrender_traits::DeviceUintSize::new(width, height);
        let dsize = webrender_traits::LayoutSize::new(width as f32, height as f32);

        let mut builder = webrender_traits::DisplayListBuilder::new(
            pipeline,
            dsize
        );

        manager.render(&mut WebBuilder {
            builder: &mut builder,
        }, width as i32, height as i32);

        self.api.set_window_parameters(
            size,
            webrender_traits::DeviceUintRect::new(
                webrender_traits::DeviceUintPoint::zero(),
                size,
            )
        );
        self.api.set_display_list(
            None,
            self.frame_id,
            dsize,
            builder.finalize(),
            false,
        );
        self.api.generate_frame(None);

        self.renderer.render(size);
    }
}

pub struct Info {

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

struct WebBuilder<'a> {
    builder: &'a mut DisplayListBuilder,
}

impl <'a> stylish::RenderVisitor<Info> for WebBuilder<'a> {
    fn visit(&mut self, obj: &mut stylish::RenderObject<Info>) {
        let color = obj.get_value::<String>("color")
            .and_then(|v| parse_color(&v))
            .unwrap_or((255, 255, 255, 0));

        let rect = webrender_traits::LayoutRect::new(
            webrender_traits::LayoutPoint::new(obj.draw_rect.x as f32, obj.draw_rect.y as f32),
            webrender_traits::LayoutSize::new(obj.draw_rect.width as f32, obj.draw_rect.height as f32),
        );

        let clip = self.builder.push_clip_region(&rect, ::std::iter::empty(), None);
        let c = webrender_traits::ColorF::new(
            color.0 as f32 / 255.0,
            color.1 as f32 / 255.0,
            color.2 as f32 / 255.0,
            color.3 as f32 / 255.0,
        );
        self.builder.push_rect(rect, clip, c);

        let clip = self.builder.push_clip_region(&rect, ::std::iter::empty(), None);
        let c = webrender_traits::ColorF::new(
            1.0 - color.0 as f32 / 255.0,
            1.0 - color.1 as f32 / 255.0,
            1.0 - color.2 as f32 / 255.0,
            color.3 as f32 / 255.0,
        );
        let border_side = webrender_traits::BorderSide {
            color: c,
            style: webrender_traits::BorderStyle::Inset,
        };
        self.builder.push_border(
            rect, clip,
            webrender_traits::BorderWidths {
                left: 1.0,
                top: 1.0,
                right: 1.0,
                bottom: 1.0,
            },
            webrender_traits::BorderDetails::Normal(
                webrender_traits::NormalBorder {
                    left: border_side,
                    right: border_side,
                    top: border_side,
                    bottom: border_side,
                    radius: webrender_traits::BorderRadius {
                        top_left: webrender_traits::LayoutSize::new(2.0, 2.0),
                        top_right: webrender_traits::LayoutSize::new(2.0, 2.0),
                        bottom_left: webrender_traits::LayoutSize::new(2.0, 2.0),
                        bottom_right: webrender_traits::LayoutSize::new(2.0, 2.0),
                    }
                }
            )
        );
    }
}

struct Dummy;
impl RenderNotifier for Dummy {
    fn new_frame_ready(&mut self) {
    }

    fn new_scroll_frame_ready(&mut self, _composite_needed: bool) {
    }
}