
pub trait Assets {
    fn load_image(&self, name: &str) -> Option<Image>;
    fn load_font(&self, name: &str) -> Option<Vec<u8>>;
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub components: Components,
    pub data: Vec<u8>,
    pub is_opaque: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Components {
    RGB,
    BGRA,
}
