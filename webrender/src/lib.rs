
extern crate webrender;
extern crate webrender_traits;
extern crate gleam;
extern crate stylish;
extern crate app_units;
extern crate stb_truetype;

mod assets;
pub use assets::*;
mod math;
mod color;
use color::*;
mod shadow;
use shadow::*;

use webrender::*;
use webrender_traits::*;
use std::error::Error;
use std::collections::HashMap;

// TODO: Don't box errors, error chain would be better
type WResult<T> = Result<T, Box<Error>>;

/// Allows for rendering a `stylish::Manager` via webrender.
///
/// # Supported Properties
///
/// * `background_color` - Set the color of the bounds
///                        of this element.
///
///    Possible values:
///
///    * `"#RRGGBB"` - **R**ed, **G**reen, **B**lue in hex.
///    * `"#RRGGBBAA"` - **R**ed, **G**reen, **B**lue, **A**lpha
///                       in hex.
///    * `rgb(R, G, B)` - **R**ed, **G**reen, **B**lue in decimal 0-255.
///    * `rgba(R, G, B, A)` - **R**ed, **G**reen, **B**lue, **A**lpha
///                        in decimal 0-255.
pub struct WebRenderer<A> {
    assets: A,
    renderer: Renderer,
    api: RenderApi,
    frame_id: Epoch,

    images: HashMap<String, ImageKey>,

    tmp_font: FontKey,
    tmp_font_info: stb_truetype::FontInfo<Vec<u8>>,
}

impl <A: Assets> WebRenderer<A> {
    pub fn new<F>(
        load_fn: F,
        assets: A,
        manager: &mut stylish::Manager<Info>,
    ) -> WResult<WebRenderer<A>>
        where F: Fn(&str) -> *const ()
    {
        let gl = unsafe {gleam::gl::GlFns::load_with(|f|
            load_fn(f) as *const _
        )};

        manager.add_func_raw("rgb", rgb);
        manager.add_func_raw("rgba", rgba);
        manager.add_func_raw("gradient", gradient);
        manager.add_func_raw("stop", stop);
        manager.add_func_raw("deg", math::deg);
        manager.add_func_raw("shadow", shadow);
        manager.add_func_raw("shadows", shadows);

        let options = webrender::RendererOptions {
            device_pixel_ratio: 1.0,
            resource_override_path: None,
            debug: false,
            clear_framebuffer: true, // TODO: Make false
            .. Default::default()
        };
        let (renderer, sender) = webrender::Renderer::new(gl, options, DeviceUintSize::new(800, 480)).unwrap();
        let api = sender.create_api();
        renderer.set_render_notifier(Box::new(Dummy));

        let pipeline = PipelineId(0, 0);
        api.set_root_pipeline(pipeline);

        let mut tmp = vec![];
        ::std::io::Read::read_to_end(&mut ::std::fs::File::open("res/indie_flower.ttf").unwrap(), &mut tmp).unwrap();

        let info = stb_truetype::FontInfo::new(tmp.clone(), 0).unwrap();

        let tmp_font = api.generate_font_key();
        api.add_raw_font(tmp_font, tmp, 0);

        Ok(WebRenderer {
            assets: assets,
            renderer: renderer,
            api: api,
            frame_id: Epoch(0),

            images: HashMap::new(),

            tmp_font: tmp_font,
            tmp_font_info: info,
        })
    }

    pub fn render(&mut self, manager: &mut stylish::Manager<Info>, width: u32, height: u32) {
        self.frame_id.0 += 1;
        let pipeline = PipelineId(0, 0);
        self.renderer.update();
        let size = DeviceUintSize::new(width, height);
        let dsize = LayoutSize::new(width as f32, height as f32);

        let mut builder = DisplayListBuilder::new(
            pipeline,
            dsize
        );

        let clip = LayoutRect::new(
            LayoutPoint::new(0.0, 0.0),
            dsize,
        );

        manager.render(&mut WebBuilder {
            api: &self.api,
            builder: &mut builder,
            assets: &mut self.assets,
            images: &mut self.images,
            font: self.tmp_font,
            font_info: &self.tmp_font_info,
            clip_rect: clip,
            offset: Vec::with_capacity(16),
        }, width as i32, height as i32);

        self.api.set_window_parameters(
            size,
            DeviceUintRect::new(
                DeviceUintPoint::zero(),
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

#[derive(Debug)]
pub struct Info {
    background_color: Option<Color>,
    image: Option<ImageKey>,
    shadows: Vec<Shadow>,
}

struct WebBuilder<'a, A: 'a> {
    api: &'a RenderApi,
    builder: &'a mut DisplayListBuilder,

    assets: &'a mut A,
    images: &'a mut HashMap<String, ImageKey>,

    font: FontKey,
    font_info: &'a stb_truetype::FontInfo<Vec<u8>>,
    clip_rect: LayoutRect,

    offset: Vec<LayoutPoint>,
}

impl <'a, A: Assets> stylish::RenderVisitor<Info> for WebBuilder<'a, A> {
    fn visit(&mut self, obj: &mut stylish::RenderObject<Info>) {
        use std::collections::hash_map::Entry;
        if obj.render_info.is_none() {
            obj.render_info = Some(Info {
                background_color: Color::get(obj, "background_color"),
                image: obj.get_value::<String>("image")
                    .and_then(|v| match self.images.entry(v) {
                        Entry::Occupied(v) => Some(*v.get()),
                        Entry::Vacant(v) => {
                            if let Some(img) = self.assets.load_image(v.key()) {
                                let key = self.api.generate_image_key();
                                self.api.add_image(
                                    key,
                                    ImageDescriptor {
                                        format: match img.components {
                                            Components::RGB => ImageFormat::RGB8,
                                            Components::RGBA => ImageFormat::RGBA8,
                                        },
                                        width: img.width,
                                        height: img.height,
                                        stride: None,
                                        offset: 0,
                                        is_opaque: match img.components {
                                            Components::RGB => true,
                                            Components::RGBA => false,
                                        },
                                    },
                                    ImageData::new(img.data),
                                    None
                                );
                                Some(*v.insert(key))
                            } else {
                                None
                            }
                        },
                    }),
                shadows: obj.get_custom_value::<Shadow>("shadow")
                    .cloned()
                    .map(|v| vec![v])
                    .or_else(|| obj.get_custom_value::<Vec<Shadow>>("shadow")
                        .cloned())
                    .unwrap_or_else(Vec::new),
            });
        }

        let info = obj.render_info.as_ref().unwrap();

        let width = obj.draw_rect.width as f32;
        let height = obj.draw_rect.height as f32;

        let offset = self.offset.last().cloned().unwrap_or(LayoutPoint::zero());

        let rect = LayoutRect::new(
            LayoutPoint::new(obj.draw_rect.x as f32 + offset.x, obj.draw_rect.y as f32 + offset.y),
            LayoutSize::new(width, height),
        );

        if let Some(key) = info.image {
            let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
            self.builder.push_image(rect, clip, rect.size, LayoutSize::zero(), ImageRendering::Auto, key);
        }

        if let Some(col) = info.background_color.as_ref() {
            match *col {
                Color::Solid(col) => {
                    let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
                    self.builder.push_rect(rect, clip, col);
                },
                Color::Gradient{angle, ref stops} => {
                    let len = width.max(height) / 2.0;
                    let mut x = len * angle.cos();
                    let mut y = len * angle.sin();
                    if x.abs() > width {
                        let s = width / x;
                        x = x.signum() * width;
                        y *= s;
                    }
                    if y.abs() > height {
                        let s = height / x;
                        y = y.signum() * height;
                        x *= s;
                    }

                    let g = self.builder.create_gradient(
                        LayoutPoint::new(width / 2.0 - x, height / 2.0 - y),
                        LayoutPoint::new(width / 2.0 + x, height / 2.0 + y),
                        stops.clone(),
                        ExtendMode::Clamp,
                    );
                    let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
                    self.builder.push_gradient(
                        rect, clip,
                        g,
                        LayoutSize::new(width, height),
                        LayoutSize::zero(),
                    );
                }
            }
        }

        for shadow in &info.shadows {
            let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
            self.builder.push_box_shadow(
                rect,
                clip,
                rect,
                shadow.offset,
                shadow.color,
                shadow.blur_radius,
                shadow.spread_radius,
                0.0,
                shadow.clip_mode,
            );
        }

        self.offset.push(rect.origin);
    }

    fn visit_end(&mut self, _obj: &mut stylish::RenderObject<Info>) {
        self.offset.pop();
    }
}
/*
        let color = obj.get_value::<String>("color")
            .and_then(|v| parse_color(&v))
            .unwrap_or((255, 255, 255, 0));
        if color.3 == 0 {
            return;
        }

        let rect = LayoutRect::new(
            LayoutPoint::new(obj.draw_rect.x as f32, obj.draw_rect.y as f32),
            LayoutSize::new(obj.draw_rect.width as f32, obj.draw_rect.height as f32),
        );

        if let Some(txt) = obj.text.as_ref() {
            let font_info = &self.font_info;
            let chars = txt.chars()
                .scan((0.0, None), |state, v| {
                    let index = font_info.find_glyph_index(v as u32);
                    let scale = font_info.scale_for_pixel_height(16.0);
                    state.0 = if let Some(last) = state.1 {
                        let kern = font_info.get_glyph_kern_advance(last, index);
                        state.0 + kern as f32 * scale
                    } else {
                        state.0
                    };
                    state.1 = Some(index);

                    let pos = state.0;
                    state.0 += font_info.get_glyph_h_metrics(index).advance_width as f32 * scale + 1.0;

                    Some(GlyphInstance {
                        index: index,
                        point: LayoutPoint::new(
                            obj.draw_rect.x as f32 + pos,
                            obj.draw_rect.y as f32
                        ),
                    })
                })
                .collect::<Vec<_>>();
            let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
            self.builder.push_text(
                rect,
                clip,
                &chars,
                self.font,
                ColorF::new(
                    color.0 as f32 / 255.0,
                    color.1 as f32 / 255.0,
                    color.2 as f32 / 255.0,
                    color.3 as f32 / 255.0,
                ),
                app_units::Au::from_px(16),
                0.0,
                None
            );
            return;
        }

        let g = self.builder.create_gradient(
            LayoutPoint::zero(),
            LayoutPoint::new(0.0, obj.draw_rect.height as f32),
            vec![
                GradientStop {
                    offset: 0.0,
                    color: ColorF::new(
                        color.0 as f32 / 255.0,
                        color.1 as f32 / 255.0,
                        color.2 as f32 / 255.0,
                        color.3 as f32 / 255.0,
                    ),
                },
                GradientStop {
                    offset: 1.0,
                    color: ColorF::new(
                        0.0, 0.0, 0.0, 1.0
                    ),
                }
            ],
            ExtendMode::Clamp,
        );
        let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
        self.builder.push_gradient(
            rect, clip,
            g,
            LayoutSize::new(obj.draw_rect.width as f32, obj.draw_rect.height as f32),
            LayoutSize::zero(),
        );

        // let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
        // let c = ColorF::new(
        //     color.0 as f32 / 255.0,
        //     color.1 as f32 / 255.0,
        //     color.2 as f32 / 255.0,
        //     color.3 as f32 / 255.0,
        // );
        // self.builder.push_rect(rect, clip, c);

        let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
        let c = ColorF::new(
            1.0 - color.0 as f32 / 255.0,
            1.0 - color.1 as f32 / 255.0,
            1.0 - color.2 as f32 / 255.0,
            color.3 as f32 / 255.0,
        );
        let border_side = BorderSide {
            color: c,
            style: BorderStyle::Inset,
        };
        self.builder.push_border(
            rect, clip,
            BorderWidths {
                left: 1.0,
                top: 1.0,
                right: 1.0,
                bottom: 1.0,
            },
            BorderDetails::Normal(
                NormalBorder {
                    left: border_side,
                    right: border_side,
                    top: border_side,
                    bottom: border_side,
                    radius: BorderRadius {
                        top_left: LayoutSize::new(2.0, 2.0),
                        top_right: LayoutSize::new(2.0, 2.0),
                        bottom_left: LayoutSize::new(2.0, 2.0),
                        bottom_right: LayoutSize::new(2.0, 2.0),
                    }
                }
            )
        );
    }
}
*/

struct Dummy;
impl RenderNotifier for Dummy {
    fn new_frame_ready(&mut self) {
    }

    fn new_scroll_frame_ready(&mut self, _composite_needed: bool) {
    }
}