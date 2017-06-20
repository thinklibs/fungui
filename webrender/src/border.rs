
use webrender_traits::*;
use stylish;
use stylish::error::ErrorKind;
use super::color::Color;

#[derive(Clone)]
pub struct BorderWidthInfo {
    pub widths: BorderWidths,
}

impl stylish::CustomValue for BorderWidthInfo {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}

pub fn border_width(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let left = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("left/width"))?
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("left/width", "float"))?;

    let top = params.get(1)
        .unwrap_or(&stylish::Value::Float(left))
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("top", "float"))?;
    let right = params.get(2)
        .unwrap_or(&stylish::Value::Float(left))
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("right", "float"))?;
    let bottom = params.get(3)
        .unwrap_or(&stylish::Value::Float(top))
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("bottom", "float"))?;

    Ok(stylish::Value::Any(Box::new(BorderWidthInfo {
        widths: BorderWidths {
            left: left as f32,
            top: top as f32,
            right: right as f32,
            bottom: bottom as f32,
        }
    })))
}

#[derive(Clone)]
pub enum Border {
    Normal {
        left: BorderSide,
        top: BorderSide,
        right: BorderSide,
        bottom: BorderSide,
    },
    Image {
        image: String,
        patch: NinePatchDescriptor,
        repeat: RepeatMode,
    }
}

impl stylish::CustomValue for Border {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}

#[derive(Clone)]
struct BSide(BorderSide);

impl stylish::CustomValue for BSide {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}

pub fn border(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let left = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("left/width"))?
        .get_custom_value::<BSide>()
        .map(|v| v.0)
        .ok_or_else(|| ErrorKind::IncorrectType("left/width", "border side"))?;

    let top = params.get(1)
        .and_then(|v| v.get_custom_value::<BSide>())
        .map(|v| v.0)
        .unwrap_or(left);

    let right = params.get(2)
        .and_then(|v| v.get_custom_value::<BSide>())
        .map(|v| v.0)
        .unwrap_or(left);
    let bottom = params.get(3)
        .and_then(|v| v.get_custom_value::<BSide>())
        .map(|v| v.0)
        .unwrap_or(top);

    Ok(stylish::Value::Any(Box::new(Border::Normal {
        left: left,
        top: top,
        right: right,
        bottom: bottom,
    })))
}


pub fn border_image(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    use euclid;
    let image = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("image"))?
        .get_value::<String>()
        .ok_or_else(|| ErrorKind::IncorrectType("image", "string"))?;

    let pwidth = params.get(1)
        .ok_or_else(|| ErrorKind::MissingParameter("width"))?
        .get_value::<i32>()
        .ok_or_else(|| ErrorKind::IncorrectType("width", "integer"))?;

    let pheight = params.get(2)
        .and_then(|v| v.get_value::<i32>())
        .unwrap_or(pwidth);

    let repeat = params.get(3)
        .and_then(|v| v.get_value::<String>())
        .map(|v| match v.as_ref() {
            "stretch" => RepeatMode::Stretch,
            "round" => RepeatMode::Round,
            "space" => RepeatMode::Space,
            _ => RepeatMode::Repeat,
        })
        .unwrap_or(RepeatMode::Repeat);

    Ok(stylish::Value::Any(Box::new(Border::Image {
        image: image,
        patch: NinePatchDescriptor {
            width: pwidth as u32 * 3,
            height: pheight as u32 * 3,
            slice: euclid::SideOffsets2D::new(pheight as u32, pwidth as u32, pheight as u32, pwidth as u32), // TODO: eh?
        },
        repeat: repeat,
    })))
}

pub fn border_side(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let color = Color::get_val(params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("color"))?)
        .ok_or_else(|| ErrorKind::IncorrectType("color", "color"))?;
    let style = params.get(1)
        .and_then(|v| v.get_value::<String>())
        .unwrap_or_else(|| "solid".to_owned());

    if let Color::Solid(col) = color {
        Ok(stylish::Value::Any(Box::new(BSide(BorderSide {
            color: col,
            style: match style.as_str() {
                "solid" => BorderStyle::Solid,
                "double" => BorderStyle::Double,
                "dotted" => BorderStyle::Dotted,
                "dashed" => BorderStyle::Dashed,
                "hidden" => BorderStyle::Hidden,
                "groove" => BorderStyle::Groove,
                "ridge" => BorderStyle::Ridge,
                "inset" => BorderStyle::Inset,
                "outset" => BorderStyle::Outset,
                _ => BorderStyle::None,
            }
        }))))
    } else {
        Err(ErrorKind::Msg("Only solid colors can be used in a normal border".into()).into())
    }
}