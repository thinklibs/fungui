
use stylish;
use stylish::error::ErrorKind;
use webrender_traits::*;
use color::*;

#[derive(Clone, Debug)]
pub struct Shadow {
    pub offset: LayoutVector2D,
    pub color: ColorF,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub clip_mode: BoxShadowClipMode,
}

impl stylish::CustomValue for Shadow {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}
pub fn shadows(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let mut shadows =params.into_iter()
        .map(|v|
            v.get_custom_value::<Shadow>()
                .map(|v| v.clone())
                .ok_or_else(|| ErrorKind::IncorrectType("shadow", "shadow").into())
        )
        .collect::<stylish::SResult<Vec<_>>>()?;

    Ok(stylish::Value::Any(Box::new(shadows)))
}

pub fn shadow(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
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
    let spread_radius = params.get(4)
        .map_or(Ok(0.0), |v| v.get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("spread_radius", "float")))?
        as f32;

    let clip_mode = params.get(5)
        .map_or(Ok("outset".to_owned()), |v| v.get_value::<String>()
        .ok_or_else(|| ErrorKind::IncorrectType("outset", "string")))?;

    let clip_mode = match clip_mode.as_str() {
        "outset" => BoxShadowClipMode::Outset,
        "inset" => BoxShadowClipMode::Inset,
        _ => return Err(ErrorKind::Msg("Expected either outset or inset".into()).into()),
    };

    Ok(stylish::Value::Any(Box::new(Shadow {
        offset: LayoutVector2D::new(offset_x as f32, offset_y as f32),
        color: color,
        blur_radius: blur_radius,
        spread_radius: spread_radius,
        clip_mode: clip_mode,
    })))
}