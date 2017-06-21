
use webrender_traits::*;
use stylish;
use stylish::error::ErrorKind;

#[derive(Clone, Debug)]
pub enum Color {
    Solid(ColorF),
    Gradient {
        angle: f32,
        stops: Vec<GradientStop>,
    },
}

impl Color {
    pub fn get<T>(obj: &stylish::RenderObject<T>, name: &str) -> Option<Color> {
        if let Some(col) = obj.get_value::<String>(name)
            .and_then(|v| parse_color(&v))
            .map(|v| Color::Solid(v))
        {
            Some(col)
        } else if let Some(col) = obj.get_custom_value::<Color>(name) {
            Some(col.clone())
        } else {
            None
        }
    }
    pub fn get_val(obj: &stylish::Value) -> Option<Color> {
        if let Some(col) = obj.get_value::<String>()
            .and_then(|v| parse_color(&v))
            .map(|v| Color::Solid(v))
        {
            Some(col)
        } else if let Some(col) = obj.get_custom_value::<Color>() {
            Some(col.clone())
        } else {
            None
        }
    }
}

impl stylish::CustomValue for Color {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}

#[derive(Clone, Debug)]
pub struct ColorStop(GradientStop);
impl stylish::CustomValue for ColorStop {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}

pub fn stop(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let offset = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("offset"))?
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("offset", "float"))?;
    let color = Color::get_val(params.get(1)
        .ok_or_else(|| ErrorKind::MissingParameter("color"))?)
        .ok_or_else(|| ErrorKind::IncorrectType("color", "color"))?;

    if let Color::Solid(col) = color {
        Ok(stylish::Value::Any(Box::new(ColorStop(GradientStop {
            offset: offset as f32,
            color: col,
        }))))
    } else {
        Err(ErrorKind::Msg("Only solid colors can be used in a gradient".into()).into())
    }
}

pub fn gradient(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let angle = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("angle"))?
        .get_value::<f64>()
        .ok_or_else(|| ErrorKind::IncorrectType("angle", "float"))?;

    let stops = params.into_iter()
        .skip(1)
        .map(|v|
            v.get_custom_value::<ColorStop>()
                .map(|v| v.0)
                .ok_or_else(|| ErrorKind::IncorrectType("stop", "color stop").into())
        )
        .collect::<stylish::SResult<Vec<_>>>()?;

    Ok(stylish::Value::Any(Box::new(Color::Gradient {
        angle: angle as f32,
        stops: stops,
    })))
}

pub fn rgb(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let r = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("r"))?;
    let g = params.get(1)
        .ok_or_else(|| ErrorKind::MissingParameter("g"))?;
    let b = params.get(2)
        .ok_or_else(|| ErrorKind::MissingParameter("b"))?;

    Ok(stylish::Value::Any(Box::new(Color::Solid(ColorF::new(
        match *r {
            stylish::Value::Integer(v) => v as f32 / 255.0,
            stylish::Value::Float(v) => v as f32,
            _ =>return Err(ErrorKind::IncorrectType("r", "float or integer").into()),
        },
        match *g {
            stylish::Value::Integer(v) => v as f32 / 255.0,
            stylish::Value::Float(v) => v as f32,
            _ =>return Err(ErrorKind::IncorrectType("g", "float or integer").into()),
        },
        match *b {
            stylish::Value::Integer(v) => v as f32 / 255.0,
            stylish::Value::Float(v) => v as f32,
            _ =>return Err(ErrorKind::IncorrectType("b", "float or integer").into()),
        },
        1.0
    )))))
}

pub fn rgba(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let r = params.get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("r"))?;
    let g = params.get(1)
        .ok_or_else(|| ErrorKind::MissingParameter("g"))?;
    let b = params.get(2)
        .ok_or_else(|| ErrorKind::MissingParameter("b"))?;
    let a = params.get(3)
        .ok_or_else(|| ErrorKind::MissingParameter("a"))?;

    Ok(stylish::Value::Any(Box::new(Color::Solid(ColorF::new(
        match *r {
            stylish::Value::Integer(v) => v as f32 / 255.0,
            stylish::Value::Float(v) => v as f32,
            _ =>return Err(ErrorKind::IncorrectType("r", "float or integer").into()),
        },
        match *g {
            stylish::Value::Integer(v) => v as f32 / 255.0,
            stylish::Value::Float(v) => v as f32,
            _ =>return Err(ErrorKind::IncorrectType("g", "float or integer").into()),
        },
        match *b {
            stylish::Value::Integer(v) => v as f32 / 255.0,
            stylish::Value::Float(v) => v as f32,
            _ =>return Err(ErrorKind::IncorrectType("b", "float or integer").into()),
        },
        match *a {
            stylish::Value::Integer(v) => v as f32 / 255.0,
            stylish::Value::Float(v) => v as f32,
            _ =>return Err(ErrorKind::IncorrectType("a", "float or integer").into()),
        },
    )))))
}

/// Parses hex and decimal color codes
pub fn parse_color(v: &str) -> Option<ColorF> {
    if v.starts_with("#") {
        let col = &v[1..];
        if col.len() == 6 || col.len() == 8 {
            Some(ColorF::new(
                u8::from_str_radix(&col[..2], 16)
                    .unwrap() as f32 / 255.0,
                u8::from_str_radix(&col[2..4], 16)
                    .unwrap() as f32 / 255.0,
                u8::from_str_radix(&col[4..6], 16)
                    .unwrap() as f32 / 255.0,
                if col.len() == 8 {
                    u8::from_str_radix(&col[6..8], 16)
                        .unwrap()
                } else { 255 } as f32 / 255.0,
            ))
        } else {
            None
        }
    } else if v.starts_with("rgb(") && v.ends_with(")") {
        let col = &v[4..v.len() - 1];
        let mut col = col.split(",").map(|v| v.trim());

        Some(ColorF::new(
            col.next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as f32 / 255.0,
            col.next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as f32 / 255.0,
            col.next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as f32 / 255.0,
            1.0,
        ))
    } else if v.starts_with("rgba(") && v.ends_with(")") {
        let col = &v[5..v.len() - 1];
        let mut col = col.split(",").map(|v| v.trim());

        Some(ColorF::new(
            col.next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as f32 / 255.0,
            col.next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as f32 / 255.0,
            col.next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as f32 / 255.0,
            col.next()
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0) as f32 / 255.0,
        ))
    } else {
        None
    }
}