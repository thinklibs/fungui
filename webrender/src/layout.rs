
use std::rc::Rc;
use stb_truetype;
use webrender_traits::RenderApi;
use stylish::{Rect, LayoutEngine, RenderObject};
use super::{
    Info,
    FontMap,
    Font,
    Assets,
};

pub(crate) struct Lined<A> {
    api: RenderApi,
    fonts: FontMap,
    assets: Rc<A>,

    line: i32,
    max_lines: i32,
    line_height: i32,
    remaining: i32,
    width: i32,
}

impl <A: Assets> Lined<A> {
    pub(crate) fn new(obj: &RenderObject<Info>, api: RenderApi, fonts: FontMap, assets: Rc<A>) -> Lined<A> {
        let height = obj.get_value::<i32>("line_height").unwrap_or(16);
        Lined {
            api: api,
            fonts: fonts,
            assets: assets,

            line: 0,
            line_height: height,
            max_lines: obj.max_size.1.unwrap_or(obj.draw_rect.height) / height,
            remaining: obj.max_size.0.unwrap_or(obj.draw_rect.width),
            width: obj.max_size.0.unwrap_or(obj.draw_rect.width),
        }
    }
}

impl <A: Assets> LayoutEngine<Info> for Lined<A> {
    fn pre_position_child(&mut self, obj: &mut RenderObject<Info>, _parent: &RenderObject<Info>) {
        if obj.text.is_some() {
            // Handled in post
            return;
        }
        let w = if self.line == self.max_lines - 1 {
            self.remaining
        } else {
            self.width
        };
        let h = self.line_height;
        obj.max_size.1 = Some(h);
        obj.max_size.0 = Some(w);
        let mut width = obj.get_value::<i32>("width")
            .unwrap_or(w);
        if width > w {
            width = w;
        }
        let mut height = obj.get_value::<i32>("height")
            .unwrap_or(h);
        if height > h {
            height = h;
        }
        obj.draw_rect = Rect {
            x: self.width - self.remaining,
            y: self.line * self.line_height + (self.line_height - height) / 2,
            width: width,
            height: height,
        };
    }
    fn post_position_child(&mut self, obj: &mut RenderObject<Info>, _parent: &RenderObject<Info>) {
        use std::collections::hash_map::Entry;
        use std::cmp;
        if let Some(txt) = obj.text.as_ref() {
            // TODO: This duplicates a lot of the text rendering code
            if let Some(font) = obj.get_value::<String>("font") {
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
                    let scale = finfo.info.scale_for_pixel_height(size as f32);

                    let mut word = (0, 0);
                    let mut word_size = 0.0;
                    let mut current = (0, 0);
                    let mut current_size = 0.0;
                    let mut last_glyph = None;
                    for (idx, c) in txt.char_indices() {
                        if c.is_whitespace() {
                            current_size += word_size;
                            word_size = 0.0;
                            current.1 = idx;
                            word.0 = idx;
                        }
                        word.1 = idx;
                        let index = finfo.info.find_glyph_index(c as u32);

                        let offset = if let Some(last) = last_glyph {
                            let kern = finfo.info.get_glyph_kern_advance(last, index);
                            kern as f32 * scale
                        } else {
                            0.0
                        };

                        let size = (finfo.info.get_glyph_h_metrics(index).advance_width as f32 * scale).ceil();
                        last_glyph = Some(index);

                        if current_size + word_size + offset + size > self.remaining as f32{
                            // Split at word
                            obj.text_splits.push((
                                current.0, current.1,
                                Rect {
                                    x: self.width - self.remaining,
                                    y: self.line * self.line_height,
                                    width: self.remaining,
                                    height: self.line_height,
                                }
                            ));
                            current.0 = word.0;
                            current.1 = word.0;
                            current_size = 0.0;
                            self.remaining = self.width;
                            self.line += 1;
                            if !c.is_whitespace() {
                                word_size += offset + size;
                            }
                        } else {
                            word_size += offset + size;
                        }

                    }
                    // Add the remaining
                    current.1 = txt.len();
                    current_size += word_size;
                    let width = current_size.ceil() as i32;
                    obj.text_splits.push((
                        current.0, current.1,
                        Rect {
                            x: self.width - self.remaining,
                            y: self.line * self.line_height,
                            width: width,
                            height: self.line_height,
                        }
                    ));
                    self.remaining -= width;

                    let mut min = (i32::max_value(), i32::max_value());
                    let mut max = (0, 0);
                    for split in &obj.text_splits {
                        min.0 = cmp::min(min.0, split.2.x);
                        min.1 = cmp::min(min.1, split.2.y);
                        max.0 = cmp::max(max.0, split.2.x +split.2.width);
                        max.1 = cmp::max(max.1, split.2.y + split.2.height);
                    }
                    obj.draw_rect = Rect {
                        x: min.0,
                        y: min.1,
                        width: max.0 - min.0,
                        height: max.1 - min.1,
                    };
                }
            }
        } else {
            if self.remaining < obj.draw_rect.width {
                self.line += 1;
                self.remaining = self.width;
            }
            obj.draw_rect.x = self.width - self.remaining;
            obj.draw_rect.y = self.line * self.line_height + (self.line_height - obj.draw_rect.height) / 2;

            self.remaining -= obj.draw_rect.width;
        }
    }

    fn finalize_layout(&mut self, obj: &mut RenderObject<Info>, children: Vec<&mut RenderObject<Info>>) {
        use std::cmp;
        let mut max = obj.min_size;
        for c in children {
            max.0 = cmp::max(max.0, c.draw_rect.x + c.draw_rect.width);
            max.1 = cmp::max(max.1, c.draw_rect.y + c.draw_rect.height);
        }
        if let Some(v) = obj.max_size.0 {
            max.0 = cmp::min(v, max.0);
        }
        if let Some(v) = obj.max_size.1 {
            max.1 = cmp::min(v, max.1);
        }
        obj.draw_rect.width = max.0;
        obj.draw_rect.height = max.1;
    }
}