
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
mod layout;

use webrender::*;
use webrender_traits::*;
use std::error::Error;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

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
    assets: Rc<A>,
    renderer: Renderer,
    api: RenderApi,
    frame_id: Epoch,

    images: HashMap<String, ImageKey>,
    fonts: FontMap,
}

type FontMap = Rc<RefCell<HashMap<String, Font>>>;

struct Font {
    key: FontKey,
    info: stb_truetype::FontInfo<Vec<u8>>,
}

impl <A: Assets + 'static> WebRenderer<A> {
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

        let fonts = Rc::new(RefCell::new(HashMap::new()));
        let assets = Rc::new(assets);

        let options = webrender::RendererOptions {
            device_pixel_ratio: 1.0,
            resource_override_path: None,
            debug: false,
            clear_framebuffer: false,
            .. Default::default()
        };
        let (renderer, sender) = webrender::Renderer::new(gl, options, DeviceUintSize::new(800, 480)).unwrap();
        let api = sender.create_api();
        renderer.set_render_notifier(Box::new(Dummy));

        let pipeline = PipelineId(0, 0);
        api.set_root_pipeline(pipeline);

        {
            let fonts = fonts.clone();
            let sender = sender.clone();
            let assets = assets.clone();
            manager.add_layout_engine("lined", move |obj| {
                Box::new(layout::Lined::new(
                    obj,
                    sender.create_api(),
                    fonts.clone(),
                    assets.clone(),
                ))
            })
        }

        Ok(WebRenderer {
            assets: assets,
            renderer: renderer,
            api: api,
            frame_id: Epoch(0),

            images: HashMap::new(),
            fonts: fonts,
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
            assets: self.assets.clone(),
            images: &mut self.images,
            fonts: self.fonts.clone(),
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

    text: Option<Text>,
}

#[derive(Debug)]
struct Text {
    glyphs: Vec<GlyphInstance>,
    font: FontKey,
    size: i32,
    color: ColorF,
}

struct WebBuilder<'a, A: 'a> {
    api: &'a RenderApi,
    builder: &'a mut DisplayListBuilder,

    assets: Rc<A>,
    images: &'a mut HashMap<String, ImageKey>,
    fonts: FontMap,

    clip_rect: LayoutRect,
    offset: Vec<LayoutPoint>,
}

impl <'a, A: Assets> stylish::RenderVisitor<Info> for WebBuilder<'a, A> {
    fn visit(&mut self, obj: &mut stylish::RenderObject<Info>) {
        use std::collections::hash_map::Entry;

        let width = obj.draw_rect.width as f32;
        let height = obj.draw_rect.height as f32;

        let offset = self.offset.last().cloned().unwrap_or(LayoutPoint::zero());

        let rect = LayoutRect::new(
            LayoutPoint::new(obj.draw_rect.x as f32 + offset.x, obj.draw_rect.y as f32 + offset.y),
            LayoutSize::new(width, height),
        );

        if obj.render_info.is_none() {
            let text = if let (Some(txt), Some(font)) = (obj.text.as_ref(), obj.get_value::<String>("font")) {
                let mut fonts = self.fonts.borrow_mut();
                let finfo = match fonts.entry(font) {
                    Entry::Occupied(v) => Some(v.into_mut()),
                    Entry::Vacant(v) => {
                        if let Some(data) = self.assets.load_font(v.key()) {
                            let info = stb_truetype::FontInfo::new(data.clone(), 0).unwrap();
                            let key = self.api.generate_font_key();
                            self.api.add_raw_font(key, data, 0);
                            Some(v.insert(Font {
                                key: key,
                                info: info,
                            }))
                        } else { None }
                    },
                };
                if let Some(finfo) = finfo {
                    let size = obj.get_value::<i32>("font_size").unwrap_or(16);
                    let color = if let Some(Color::Solid(col)) = Color::get(obj, "font_color") {
                        col
                    } else {
                        ColorF::new(0.0, 0.0, 0.0, 1.0)
                    };

                    if obj.text_splits.is_empty() {
                        obj.text_splits.push((0, txt.len(), obj.draw_rect));
                    }

                    let scale = finfo.info.scale_for_pixel_height(size as f32);
                    let glyphs = obj.text_splits.iter()
                        .flat_map(|&(s, e, rect)| {
                            let rect = rect;
                            let finfo = &finfo;
                            txt[s..e].chars()
                                .scan((0.0, None), move |state, v| {
                                    let index = finfo.info.find_glyph_index(v as u32);
                                    state.0 = if let Some(last) = state.1 {
                                        let kern = finfo.info.get_glyph_kern_advance(last, index);
                                        state.0 + kern as f32 * scale
                                    } else {
                                        state.0
                                    };
                                    state.1 = Some(index);

                                    let pos = state.0;
                                    state.0 += (finfo.info.get_glyph_h_metrics(index).advance_width as f32 * scale).ceil();

                                    Some(GlyphInstance {
                                        index: index,
                                        point: LayoutPoint::new(
                                            rect.x as f32 + offset.x + pos,
                                            rect.y as f32 + offset.y + size as f32,
                                        ),
                                    })
                                })
                        })
                        .collect();
                    Some(Text {
                        glyphs: glyphs,
                        font: finfo.key,
                        size: size,
                        color: color,
                    })
                } else {
                    None
                }
            } else {
                None
            };

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
                text: text,
            });
        }

        let info = obj.render_info.as_ref().unwrap();

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

        if let Some(txt) = info.text.as_ref() {
            let clip = self.builder.push_clip_region(&self.clip_rect, None, None);
            self.builder.push_text(
                rect,
                clip,
                &txt.glyphs,
                txt.font,
                txt.color,
                app_units::Au::from_px(txt.size),
                0.0,
                None
            );
        }

        self.offset.push(rect.origin);
    }

    fn visit_end(&mut self, _obj: &mut stylish::RenderObject<Info>) {
        self.offset.pop();
    }
}

struct Dummy;
impl RenderNotifier for Dummy {
    fn new_frame_ready(&mut self) {
    }

    fn new_scroll_frame_ready(&mut self, _composite_needed: bool) {
    }
}