
use stylish;
use stylish::error::ErrorKind;

pub fn deg(params: Vec<stylish::Value>) -> stylish::SResult<stylish::Value> {
    let val = params
        .get(0)
        .ok_or_else(|| ErrorKind::MissingParameter("degrees"))?;

    if let Some(d) = val.get_value::<i32>() {
        Ok(stylish::Value::Float((d as f64).to_radians()))
    } else if let Some(d) = val.get_value::<f64>() {
        Ok(stylish::Value::Float(d.to_radians()))
    } else {
        Err(ErrorKind::IncorrectType("degrees", "float or integer").into())
    }
}
