
use webrender::api::*;
use stylish;
use stylish::error::ErrorKind;

#[derive(Clone)]
pub struct Filters(pub Vec<FilterOp>);

impl stylish::CustomValue for Filters {
    fn clone(&self) -> Box<stylish::CustomValue> {
        Box::new(Clone::clone(self))
    }
}

pub fn filters(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let mut filters = Vec::with_capacity(params.len() / 2);
    for pair in params.chunks(2) {
        if pair.len() != 2 {
            break;
        }
        let op = pair.get(1)
            .and_then(|v| v.get_value::<f64>())
            .ok_or_else(|| ErrorKind::IncorrectType("op value", "float"))?
            as f32;

        let filter = pair.get(0)
            .and_then(|v| v.get_value::<String>())
            .map(|v| match v.as_ref() {
                "blur" => Ok(FilterOp::Blur(op)),
                "brightness" => Ok(FilterOp::Brightness(op)),
                "contrast" => Ok(FilterOp::Contrast(op)),
                "grayscale" => Ok(FilterOp::Grayscale(op)),
                "hue_rotate" => Ok(FilterOp::HueRotate(op)),
                "invert" => Ok(FilterOp::Invert(op)),
                "opacity" => Ok(FilterOp::Opacity(PropertyBinding::Value(op))),
                "saturate" => Ok(FilterOp::Saturate(op)),
                "sepia" => Ok(FilterOp::Sepia(op)),
                _ => Err(ErrorKind::Msg("Invalid filter".into())),
            })
            .ok_or_else(|| ErrorKind::IncorrectType("filter", "string"))
            .and_then(|v| v)?;

        filters.push(filter);
    }

    Ok(stylish::Value::Any(Box::new(Filters(filters))))
}