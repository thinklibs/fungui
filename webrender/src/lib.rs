
extern crate app_units;
extern crate euclid;
extern crate gleam;
extern crate stb_truetype;
extern crate stylish;
extern crate webrender;

mod assets;
pub use assets::*;
mod math;
mod color;
use color::*;
mod shadow;
use shadow::*;
mod text_shadow;
use text_shadow::*;
mod layout;
mod border;
mod filter;

use webrender::*;
use webrender::api::*;
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
    renderer: Option<Renderer>,
    api: RenderApi,
    document: DocumentId,
    frame_id: Epoch,

    resources: ResourceUpdates,
    images: HashMap<String, (ImageKey, ImageDescriptor)>,
    fonts: FontMap,

    skip_build: bool,
    force_build: bool,
    last_size: DeviceUintSize,
}

impl<A> Drop for WebRenderer<A> {
    fn drop(&mut self) {
        self.renderer.take().unwrap().deinit();
    }
}

type FontMap = Rc<RefCell<HashMap<String, Font>>>;

struct Font {
    key: FontKey,
    info: stb_truetype::FontInfo<Vec<u8>>,
    instances: HashMap<app_units::Au, FontInstanceKey>,
}

impl<A: Assets + 'static> WebRenderer<A> {
    pub fn new<F>(
        load_fn: F,
        assets: A,
        manager: &mut stylish::Manager<Info>,
    ) -> WResult<WebRenderer<A>>
    where
        F: Fn(&str) -> *const (),
    {
        let gl = unsafe { gleam::gl::GlFns::load_with(|f| load_fn(f) as *const _) };

        manager.add_func_raw("rgb", rgb);
        manager.add_func_raw("rgba", rgba);
        manager.add_func_raw("gradient", gradient);
        manager.add_func_raw("stop", stop);
        manager.add_func_raw("deg", math::deg);
        manager.add_func_raw("shadow", shadow);
        manager.add_func_raw("shadows", shadows);
        manager.add_func_raw("border", border::border);
        manager.add_func_raw("bside", border::border_side);
        manager.add_func_raw("border_width", border::border_width);
        manager.add_func_raw("border_image", border::border_image);
        manager.add_func_raw("filters", filter::filters);
        manager.add_func_raw("text_shadow", text_shadow);

        let fonts = Rc::new(RefCell::new(HashMap::new()));
        let assets = Rc::new(assets);

        let options = webrender::RendererOptions {
            device_pixel_ratio: 1.0,
            resource_override_path: None,
            clear_color: None,
            ..Default::default()
        };
        let (renderer, sender) = webrender::Renderer::new(gl, Box::new(Dummy), options).unwrap();
        let api = sender.create_api();
        let size = DeviceUintSize::new(800, 480);
        let document = api.add_document(size, 0);

        let pipeline = PipelineId(0, 0);
        let mut trans = Transaction::new();
        trans.set_root_pipeline(pipeline);
        api.send_transaction(document, trans);

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
            });
        }
        manager.add_layout_engine("grid", |obj| Box::new(layout::Grid::new(obj)));

        Ok(WebRenderer {
            assets: assets,
            renderer: Some(renderer),
            api: api,
            frame_id: Epoch(0),
            document: document,

            resources: ResourceUpdates::new(),
            images: HashMap::new(),
            fonts: fonts,
            skip_build: false,
            force_build: false,
            last_size: size,
        })
    }

    pub fn update_image(&mut self, key: &str, img: Image) {
        use std::collections::hash_map::Entry;
        match self.images.entry(key.to_owned()) {
            Entry::Occupied(val) => {
                let (key, desc) = *val.get();
                self.resources
                    .update_image(key, desc, ImageData::new(img.data), None);
            }
            Entry::Vacant(val) => {
                let key = self.api.generate_image_key();
                let desc = ImageDescriptor {
                    format: match img.components {
                        Components::BGRA => ImageFormat::BGRA8,
                    },
                    width: img.width,
                    height: img.height,
                    stride: None,
                    offset: 0,
                    is_opaque: img.is_opaque,
                    allow_mipmaps: false,
                };
                self.resources
                    .add_image(key, desc, ImageData::new(img.data), None);
                val.insert((key, desc));
            }
        };
        self.force_build = true;
    }

    pub fn layout(&mut self, manager: &mut stylish::Manager<Info>, width: u32, height: u32) {
        if manager.layout(width as i32, height as i32) {
            self.skip_build = false;
        } else {
            self.skip_build = true;
        }
    }

    pub fn render(&mut self, manager: &mut stylish::Manager<Info>, width: u32, height: u32) {
        use std::mem::replace;
        self.frame_id.0 += 1;
        let pipeline = PipelineId(0, 0);
        self.renderer.as_mut().unwrap().update();
        let size = DeviceUintSize::new(width, height);
        let dsize = LayoutSize::new(width as f32, height as f32);

        // BUG: Currently have to rebuild every frame to work around
        //      a crash on SteamOS
        {
            self.last_size = size;
            // BUG: Webrender seems to clear fonts on re-size?
            self.fonts.borrow_mut().clear();
            self.force_build = true;
        }

        if !self.skip_build || self.force_build {
            self.force_build = false;
            let mut builder = DisplayListBuilder::new(pipeline, dsize);

            let mut resources = replace(&mut self.resources, ResourceUpdates::new());

            manager.render(&mut WebBuilder {
                api: &self.api,
                builder: &mut builder,
                assets: self.assets.clone(),
                images: &mut self.images,
                fonts: self.fonts.clone(),
                offset: Vec::with_capacity(16),
                resources: &mut resources,
            });

            let mut trans = Transaction::new();
            trans.set_window_parameters(
                size,
                DeviceUintRect::new(DeviceUintPoint::zero(), size),
                1.0,
            );
            trans.update_resources(resources);
            trans.set_display_list(
                self.frame_id,
                None,
                dsize,
                builder.finalize(),
                false,
            );
            trans.generate_frame();
            self.api.send_transaction(self.document, trans);
        }

        self.renderer.as_mut().unwrap().render(size).unwrap();
        self.skip_build = false;
    }
}

#[derive(Debug)]
pub struct Info {
    background_color: Option<Color>,
    image: Option<ImageKey>,
    shadows: Vec<shadow::Shadow>,

    text: Option<Text>,
    text_shadow: Option<TShadow>,

    border_widths: BorderWidths,
    border: Option<BorderDetails>,

    clip_id: Option<ClipId>,
    clip_overflow: bool,

    scroll_offset: LayoutVector2D,
    filters: Vec<FilterOp>,
}

#[derive(Debug)]
struct Text {
    glyphs: Vec<GlyphInstance>,
    font: FontInstanceKey,
    size: i32,
    color: ColorF,
}

struct WebBuilder<'a, A: 'a> {
    api: &'a RenderApi,
    builder: &'a mut DisplayListBuilder,
    resources: &'a mut ResourceUpdates,

    assets: Rc<A>,
    images: &'a mut HashMap<String, (ImageKey, ImageDescriptor)>,
    fonts: FontMap,

    offset: Vec<LayoutPoint>,
}

impl<'a, A: Assets> stylish::RenderVisitor<Info> for WebBuilder<'a, A> {
    fn visit(&mut self, obj: &mut stylish::RenderObject<Info>) {
        use std::collections::hash_map::Entry;

        let width = obj.draw_rect.width as f32;
        let height = obj.draw_rect.height as f32;

        let offset = self.offset.last().cloned().unwrap_or(LayoutPoint::zero());

        let rect = LayoutRect::new(
            LayoutPoint::new(
                obj.draw_rect.x as f32 + offset.x,
                obj.draw_rect.y as f32 + offset.y,
            ),
            LayoutSize::new(width, height),
        );
        let pinfo = PrimitiveInfo::new(rect);

        if obj.render_info.is_none() {
            let text = if let (Some(txt), Some(font)) =
                (obj.text.as_ref(), obj.get_value::<String>("font"))
            {
                let mut fonts = self.fonts.borrow_mut();
                let finfo = match fonts.entry(font) {
                    Entry::Occupied(v) => Some(v.into_mut()),
                    Entry::Vacant(v) => if let Some(data) = self.assets.load_font(v.key()) {
                        let info = stb_truetype::FontInfo::new(data.clone(), 0).unwrap();
                        let key = self.api.generate_font_key();
                        self.resources.add_raw_font(key, data, 0);
                        Some(v.insert(Font {
                            key: key,
                            info: info,
                            instances: HashMap::new(),
                        }))
                    } else {
                        None
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

                    let font_size = app_units::Au::from_f64_px(size as f64 * 0.8);
                    let api = &mut self.api;
                    let resources = &mut self.resources;
                    let font_key = finfo.key;
                    let font_instance = finfo.instances.entry(font_size).or_insert_with(|| {
                        let key = api.generate_font_instance_key();
                        resources.add_font_instance(key, font_key, font_size, None, None, vec![]);
                        key
                    });

                    let font_info = &finfo.info;

                    let scale = finfo.info.scale_for_pixel_height(size as f32);
                    let glyphs = obj.text_splits
                        .iter()
                        .flat_map(|&(s, e, rect)| {
                            let rect = rect;
                            txt[s..e].chars().scan((0.0, None), move |state, v| {
                                let index = font_info.find_glyph_index(v as u32);
                                let g_size = if let Some(last) = state.1 {
                                    let kern = font_info.get_glyph_kern_advance(last, index);
                                    kern as f32 * scale
                                } else {
                                    0.0
                                };
                                state.1 = Some(index);

                                let pos = state.0 + g_size;
                                state.0 += g_size
                                    + font_info.get_glyph_h_metrics(index).advance_width as f32
                                        * scale;

                                Some(GlyphInstance {
                                    index: index,
                                    point: LayoutPoint::new(
                                        rect.x as f32 + offset.x + pos,
                                        rect.y as f32 + offset.y + size as f32 * 0.8,
                                    ),
                                })
                            })
                        })
                        .collect();
                    Some(Text {
                        glyphs: glyphs,
                        font: *font_instance,
                        size: size,
                        color: color,
                    })
                } else {
                    None
                }
            } else {
                None
            };

            let mut load_image = |v| match self.images.entry(v) {
                Entry::Occupied(v) => Some(v.get().0),
                Entry::Vacant(v) => if let Some(img) = self.assets.load_image(v.key()) {
                    let key = self.api.generate_image_key();
                    let desc = ImageDescriptor {
                        format: match img.components {
                            Components::BGRA => ImageFormat::BGRA8,
                        },
                        width: img.width,
                        height: img.height,
                        stride: None,
                        offset: 0,
                        is_opaque: img.is_opaque,
                        allow_mipmaps: false,
                    };
                    self.resources
                        .add_image(key, desc, ImageData::new(img.data), None);
                    Some(v.insert((key, desc)).0)
                } else {
                    None
                },
            };

            obj.render_info = Some(Info {
                background_color: Color::get(obj, "background_color"),
                image: obj.get_value::<String>("image").and_then(|v| load_image(v)),
                shadows: obj.get_custom_value::<shadow::Shadow>("shadow")
                    .cloned()
                    .map(|v| vec![v])
                    .or_else(|| {
                        obj.get_custom_value::<Vec<shadow::Shadow>>("shadow")
                            .cloned()
                    })
                    .unwrap_or_else(Vec::new),
                text: text,
                text_shadow: obj.get_custom_value::<TShadow>("text_shadow").cloned(),

                border_widths: obj.get_custom_value::<border::BorderWidthInfo>("border_width")
                    .map(|v| v.widths)
                    .unwrap_or(BorderWidths {
                        left: 0.0,
                        top: 0.0,
                        right: 0.0,
                        bottom: 0.0,
                    }),
                border: obj.get_custom_value::<border::Border>("border")
                    .map(|v| match *v {
                        border::Border::Normal {
                            left,
                            top,
                            right,
                            bottom,
                        } => BorderDetails::Normal(NormalBorder {
                            left: left,
                            top: top,
                            right: right,
                            bottom: bottom,

                            radius: BorderRadius::uniform(
                                obj.get_value::<f64>("border_radius").unwrap_or(0.0) as f32,
                            ),
                        }),
                        border::Border::Image {
                            ref image,
                            patch,
                            repeat,
                            fill,
                        } => BorderDetails::Image(ImageBorder {
                            image_key: load_image(image.clone()).unwrap(),
                            patch: patch,
                            fill: fill,
                            outset: euclid::SideOffsets2D::new(0.0, 0.0, 0.0, 0.0),
                            repeat_horizontal: repeat,
                            repeat_vertical: repeat,
                        }),
                    }),

                clip_id: None,
                clip_overflow: obj.clip_overflow,
                scroll_offset: LayoutVector2D::new(
                    obj.scroll_position.0 as f32,
                    obj.scroll_position.1 as f32,
                ),

                filters: obj.get_custom_value::<filter::Filters>("filters")
                    .map(|v| v.0.clone())
                    .unwrap_or_default(),
            });
        }

        let info = obj.render_info.as_mut().unwrap();

        if !info.filters.is_empty() {
            self.builder.push_stacking_context(
                &PrimitiveInfo::new(LayoutRect::new(LayoutPoint::zero(), LayoutSize::zero())),
                None,
                ScrollPolicy::Scrollable,
                None,
                TransformStyle::Flat,
                None,
                MixBlendMode::Normal,
                info.filters.clone(),
            );
        }

        if let Some(key) = info.image {
            self.builder.push_image(
                &pinfo,
                rect.size,
                LayoutSize::zero(),
                ImageRendering::Auto,
                AlphaType::PremultipliedAlpha,
                key,
            );
        }

        if let Some(col) = info.background_color.as_ref() {
            match *col {
                Color::Solid(col) => {
                    self.builder.push_rect(&pinfo, col);
                }
                Color::Gradient { angle, ref stops } => {
                    let len = width.max(height) / 2.0;
                    let x = len * angle.cos();
                    let y = len * angle.sin();

                    let g = self.builder.create_gradient(
                        LayoutPoint::new(width / 2.0 - x, height / 2.0 - y),
                        LayoutPoint::new(width / 2.0 + x, height / 2.0 + y),
                        stops.clone(),
                        ExtendMode::Clamp,
                    );
                    self.builder.push_gradient(
                        &pinfo,
                        g,
                        LayoutSize::new(width, height),
                        LayoutSize::zero(),
                    );
                }
            }
        }

        if let Some(border) = info.border {
            self.builder.push_border(&pinfo, info.border_widths, border);
        }

        if let Some(txt) = info.text.as_ref() {
            if let Some(ts) = info.text_shadow.as_ref() {
                let shadow_rect = rect.translate(&ts.offset)
                    .inflate(ts.blur_radius, ts.blur_radius);
                self.builder.push_shadow(
                    &PrimitiveInfo::new(shadow_rect),
                    Shadow {
                        offset: ts.offset,
                        color: ts.color,
                        blur_radius: ts.blur_radius,
                    },
                );
            }
            self.builder
                .push_text(&pinfo, &txt.glyphs, txt.font, txt.color, None);
            if info.text_shadow.is_some() {
                self.builder.pop_all_shadows();
            }
        }

        for shadow in &info.shadows {
            self.builder.push_box_shadow(
                &PrimitiveInfo::with_clip_rect(
                    rect,
                    rect.inflate(shadow.blur_radius, shadow.blur_radius)
                        .translate(&shadow.offset),
                ),
                rect,
                shadow.offset,
                shadow.color,
                shadow.blur_radius,
                shadow.spread_radius,
                BorderRadius::zero(),
                shadow.clip_mode,
            );
        }

        info.clip_id = if info.clip_overflow {
            let id = self.builder.define_scroll_frame(
                None,
                rect,
                rect,
                None,
                None,
                ScrollSensitivity::ScriptAndInputEvents,
            );
            self.builder.push_clip_id(id);
            Some(id)
        } else {
            None
        };

        self.offset.push(rect.origin + info.scroll_offset);
    }

    fn visit_end(&mut self, obj: &mut stylish::RenderObject<Info>) {
        let info = obj.render_info.as_mut().unwrap();
        if let Some(_clip_id) = info.clip_id {
            self.builder.pop_clip_id();
        }
        if !info.filters.is_empty() {
            self.builder.pop_stacking_context();
        }
        self.offset.pop();
    }
}

struct Dummy;
impl RenderNotifier for Dummy {
    fn wake_up(&self) {}

    fn new_document_ready(&self, _id: DocumentId, _scrolled: bool, _composite_needed: bool) {}

    fn clone(&self) -> Box<RenderNotifier> {
        Box::new(Dummy)
    }
}
