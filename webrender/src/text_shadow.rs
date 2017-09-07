
use stylish;
use stylish::error::ErrorKind;
use webrender::api::*;
use color::*;
use euclid::TypedVector2D;

#[derive(Clone, Debug)]
pub struct TShadow {
    pub offset: TypedVector2D<f32, LayerPixel>,
    pub color: ColorF,
    pub blur_radius: f32,
}

impl stylish::CustomValue for TShadow {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}


pub fn text_shadow(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let offset_x = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("offset x"))?
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("offset x", "float"))?;
    let offset_y = params.get(1)
        .ok_or_else(|| ErrorKind::MissingParameter("offset y"))?
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("offset y", "float"))?;

    let color = Color::get_val(params.get(2)
        .ok_or_else(|| ErrorKind::MissingParameter("color"))?)
        .ok_or_else(|| ErrorKind::IncorrectType("color", "color"))?;
    let color = if let Color::Solid(col) = color {
        col
    } else {
        return Err(ErrorKind::Msg("Only solid colors can be used in a gradient".into()).into())
    };

    let blur_radius = params.get(3)
        .map_or(Ok(1.0), |v| v.get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("blur_radius", "float")))?
        as f32;

    Ok(stylish::Value::Any(Box::new(TShadow {
        offset: TypedVector2D::new(offset_x as f32, offset_y as f32),
        color: color,
        blur_radius: blur_radius,
    })))
}