
pub trait Assets {
    fn load_image(&mut self, name: &str) -> Option<Image>;
    fn load_font(&mut self, name: &str) -> Option<Vec<u8>>;
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub components: Components,
    pub data: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
pub enum Components {
    RGB,
    RGBA,
}