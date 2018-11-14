use super::*;
use std::fmt::{Formatter, Result as FResult, Display};

#[derive(Debug)]
pub enum RectPart {
    Width,
    Height,
}

pub enum Expr<E: Extension> {
    Value(Value<E>),
    Variable(String),
    ParentRect(RectPart),
    VariableParent(usize, String),

    Neg(Box<Expr<E>>),
    Not(Box<Expr<E>>),
    And(Box<Expr<E>>, Box<Expr<E>>),
    Or(Box<Expr<E>>, Box<Expr<E>>),
    Xor(Box<Expr<E>>, Box<Expr<E>>),

    Equal(Box<Expr<E>>, Box<Expr<E>>),
    NotEqual(Box<Expr<E>>, Box<Expr<E>>),
    LessEqual(Box<Expr<E>>, Box<Expr<E>>),
    GreaterEqual(Box<Expr<E>>, Box<Expr<E>>),
    Less(Box<Expr<E>>, Box<Expr<E>>),
    Greater(Box<Expr<E>>, Box<Expr<E>>),

    Add(Box<Expr<E>>, Box<Expr<E>>),
    Sub(Box<Expr<E>>, Box<Expr<E>>),
    Mul(Box<Expr<E>>, Box<Expr<E>>),
    Div(Box<Expr<E>>, Box<Expr<E>>),
    Rem(Box<Expr<E>>, Box<Expr<E>>),

    IntToFloat(Box<Expr<E>>),
    FloatToInt(Box<Expr<E>>),

    Call(StaticKey, Vec<Expr<E>>),
}

impl <E> Display for Expr<E>
    where E: Extension
{
    fn fmt(&self, f: &mut Formatter) -> FResult {
        match self {
            Expr::Value(Value::Boolean(v)) => write!(f, "{}", v),
            Expr::Value(Value::Integer(v)) => write!(f, "{}", v),
            Expr::Value(Value::Float(v)) => write!(f, "{}", v),
            Expr::Value(Value::String(v)) => write!(f, "{:?}", v),
            Expr::Value(Value::ExtValue(_)) => write!(f, "EXT"),
            Expr::Variable(var) => write!(f, "{}", var),
            Expr::VariableParent(d, var) => write!(f, "{}({})", var, d),
            Expr::ParentRect(part) => write!(f, "parent({:?})", part),

            Expr::Neg(e) => write!(f, "-({})", e),
            Expr::Not(e) => write!(f, "!({})", e),
            Expr::And(a, b) => write!(f, "({} && {})", a, b),
            Expr::Or(a, b) => write!(f, "({} || {})", a, b),
            Expr::Xor(a, b) => write!(f, "({} ^ {})", a, b),

            Expr::Equal(a, b) => write!(f, "({} == {})", a, b),
            Expr::NotEqual(a, b) => write!(f, "({} != {})", a, b),
            Expr::LessEqual(a, b) => write!(f, "({} <= {})", a, b),
            Expr::GreaterEqual(a, b) => write!(f, "({} >= {})", a, b),
            Expr::Less(a, b) => write!(f, "({} < {})", a, b),
            Expr::Greater(a, b) => write!(f, "({} > {})", a, b),

            Expr::Add(a, b) => write!(f, "({} + {})", a, b),
            Expr::Sub(a, b) => write!(f, "({} - {})", a, b),
            Expr::Mul(a, b) => write!(f, "({} * {})", a, b),
            Expr::Div(a, b) => write!(f, "({} / {})", a, b),
            Expr::Rem(a, b) => write!(f, "({} % {})", a, b),

            Expr::IntToFloat(e) => write!(f, "float({})", e),
            Expr::FloatToInt(e) => write!(f, "int({})", e),

            Expr::Call(name, exprs) => {
                write!(f, "{}(", name.0)?;
                for e in exprs {
                    write!(f, "{}, ", e)?;
                }
                write!(f, ")")
            },
        }
    }
}

fn get_ty<E: Extension>(v: &Value<E>) -> &'static str {
    match v {
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Boolean(_) => "boolean",
        Value::String(_) => "string",
        Value::ExtValue(_) => "extension value",
    }
}

impl <E> Expr<E>
    where E: Extension
{
    pub fn eval<'a>(&'a self, styles: &'a Styles<E>, node: &'a NodeChain<E>) -> Result<Value<E>, Error<'a>> {
        Ok(match *self {
            Expr::Value(ref v) => v.clone(),
            Expr::Variable(ref n) => return node.properties.get(n).cloned().ok_or(Error::UnknownVariable{name: n}),
            Expr::VariableParent(depth, ref n) => {
                let mut node = node;
                for _ in 0 .. depth {
                    node = node.parent.expect("Missing parent, shouldn't happen");
                }
                return node.properties.get(n).cloned().ok_or(Error::UnknownVariable{name: n});
            },
            Expr::ParentRect(RectPart::Width) => return node.parent
                .ok_or(Error::CustomStatic{reason: "No parent"})
                .map(|v| v.draw_rect.width)
                .map(Value::Integer),
            Expr::ParentRect(RectPart::Height) => return node.parent
                .ok_or(Error::CustomStatic{reason: "No parent"})
                .map(|v| v.draw_rect.height)
                .map(Value::Integer),
            Expr::Neg(ref e) => match e.eval(styles, node)? {
                Value::Integer(a) => Value::Integer(-a),
                Value::Float(a) => Value::Float(-a),
                v => return Err(Error::IncompatibleTypeOp{op: "-", ty: get_ty(&v)}),
            },
            Expr::Not(ref e) => match e.eval(styles, node)? {
                Value::Boolean(a) => Value::Boolean(!a),
                v => return Err(Error::IncompatibleTypeOp{op: "-", ty: get_ty(&v)}),
            },
            Expr::IntToFloat(ref e) => match e.eval(styles, node)? {
                Value::Integer(a) => Value::Float(a as f64),
                v => return Err(Error::IncompatibleTypeOp{op: "-", ty: get_ty(&v)}),
            },
            Expr::FloatToInt(ref e) => match e.eval(styles, node)? {
                Value::Float(a) => Value::Integer(a as i32),
                v => return Err(Error::IncompatibleTypeOp{op: "-", ty: get_ty(&v)}),
            },

            Expr::And(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a && b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "&&", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Or(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a || b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "||", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Xor(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a ^ b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "^", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },

            Expr::Equal(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a == b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "==", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::NotEqual(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a != b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "!=", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::LessEqual(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a <= b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "<=", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::GreaterEqual(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a >= b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: ">=", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Less(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a < b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "<", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Greater(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Boolean(a), Value::Boolean(b)) => Value::Boolean(a > b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: ">", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },

            Expr::Add(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "+", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Sub(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a - b),
                (Value::Float(a), Value::Float(b)) => Value::Float(a - b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "-", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Mul(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a * b),
                (Value::Float(a), Value::Float(b)) => Value::Float(a * b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "*", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Div(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a / b),
                (Value::Float(a), Value::Float(b)) => Value::Float(a / b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "/", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Rem(ref a, ref b) => match (a.eval(styles, node)?, b.eval(styles, node)?) {
                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a % b),
                (Value::Float(a), Value::Float(b)) => Value::Float(a % b),
                (a,b) => return Err(Error::IncompatibleTypesOp{op: "%", left_ty: get_ty(&a), right_ty: get_ty(&b)}),
            },
            Expr::Call(ref name, ref args) => {
                let func = styles.funcs.get(name).expect("Missing func");

                let mut args = args.iter()
                    .map(move |v| v.eval(styles, node));
                return func(&mut args)
            }
        })
    }

    pub fn from_style<'a>(
        static_keys: &FnvHashMap<&'static str, StaticKey>,
        replacements: &FnvHashMap<String, (usize, String)>,
        uses_parent_size: &mut bool,
        e: syntax::style::ExprType<'a>
    ) -> Result<Expr<E>, syntax::PError<'a>> {
        use syntax::style::Expr as SExpr;
        use syntax::style::Value as SVal;
        Ok(match e.expr {
            SExpr::Value(v) => match v {
                SVal::Boolean(b) => Expr::Value(Value::Boolean(b)),
                SVal::Integer(i) => Expr::Value(Value::Integer(i)),
                SVal::Float(f) => Expr::Value(Value::Float(f)),
                SVal::String(s) => Expr::Value(Value::String(unescape(s))),
                SVal::Variable(v) => if let Some(r) = replacements.get(v.name) {
                    if r.0 == 0 {
                        Expr::Variable(r.1.clone())
                    } else {
                        Expr::VariableParent(r.0, r.1.clone())
                    }
                } else {
                    *uses_parent_size = true;
                    match v.name {
                        "parent_width" => Expr::ParentRect(RectPart::Width),
                        "parent_height" => Expr::ParentRect(RectPart::Height),
                        _ => return Err(syntax::Errors::new(
                            v.position.into(),
                            syntax::Error::Message(syntax::Info::Borrowed("Unknown variable")),
                        ))
                    }
                },
            },
            SExpr::Neg(e) => Expr::Neg(Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *e)?)),

            SExpr::Not(e) => Expr::Not(Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *e)?)),
            SExpr::And(l, r) => Expr::And(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Or(l, r) => Expr::Or(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Xor(l, r) => Expr::Xor(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),

            SExpr::Add(l, r) => Expr::Add(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Sub(l, r) => Expr::Sub(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Mul(l, r) => Expr::Mul(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Div(l, r) => Expr::Div(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Rem(l, r) => Expr::Rem(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),

            SExpr::Equal(l, r) => Expr::Equal(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::NotEqual(l, r) => Expr::NotEqual(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::LessEqual(l, r) => Expr::LessEqual(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::GreaterEqual(l, r) => Expr::GreaterEqual(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Less(l, r) => Expr::Less(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),
            SExpr::Greater(l, r) => Expr::Greater(
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *l)?),
                Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *r)?),
            ),

            SExpr::IntToFloat(e) => Expr::IntToFloat(Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *e)?)),
            SExpr::FloatToInt(e) => Expr::FloatToInt(Box::new(Expr::from_style(static_keys, replacements, uses_parent_size, *e)?)),

            SExpr::Call(name, params) => {
                let key = static_keys.get(name.name).ok_or_else(|| {
                    syntax::Errors::new(
                        name.position.into(),
                        syntax::Error::Message(syntax::Info::Borrowed("Unknown function")),
                    )
                })?;
                Expr::Call(*key, params.into_iter()
                    .map(|v| Expr::from_style(static_keys, replacements, uses_parent_size, v))
                    .collect::<Result<Vec<_>, _>>()?
                )
            },

        })
    }
}