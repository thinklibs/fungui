//! Parser for the UI style format
//!
//! This module contains the AST and parser for the
//! format used to style and position a UI element.
//!
//! The format is as follows:
//!
//! ```text,ignore
//! // Comments (only single line)
//!
//! // Name of an element. Can be made up from any
//! // letter, number or _
//! root > panel > image(width=width, height=height) {
//!     width = width,
//!     height = height,
//! }
//! emoji(type="smile") {
//!     image = "icons/smile.png",
//! }
//! panel > @text {
//!     color = "#0050AA",
//! }
//! ```

use combine::*;
use combine::char::{char, digit, alpha_num, spaces, string, space};
use combine::primitives::{Error, SourcePosition};
use std::collections::HashMap;
use std::fmt::Debug;
use super::{
    Ident,
    Position,
};

/// A UI style document
#[derive(Debug)]
pub struct Document {
    /// A list of rules in this document
    pub rules: Vec<Rule>
}

impl Document {
    /// Attempts to parse the given string as a document.
    ///
    /// This fails when a syntax error occurs. The returned
    /// error can be formatted in a user friendly format
    /// via the [`format_parse_error`] method.
    ///
    /// # Example
    ///
    /// ```
    /// # use stylish_syntax::style::Document;
    /// assert!(Document::parse(r##"
    /// panel {
    ///     background = "#ff0000",
    /// }
    /// "##).is_ok());
    /// ```
    ///
    /// [`format_parse_error`]: ../fn.format_parse_error.html
    pub fn parse(source: &str) -> Result<Document, ParseError<State<&str>>> {
        let (doc, _) = parser(parse_document).parse(State::new(source))?;
        Ok(doc)
    }
}

#[derive(Debug)]
pub struct Rule {
    pub matchers: Vec<Matcher>,
    pub styles: HashMap<Ident, ExprType>,
}

#[derive(Debug)]
pub enum Matcher {
    Element(Element),
    Text,
}

/// An element which can contain other elements and/or
/// have properties attached.
///
/// An element does nothing by itself (bar special elements
/// as defined by the program, widgets) and must be controlled
/// via a style document.
#[derive(Debug)]
pub struct Element {
    /// The name of this element
    pub name: Ident,
    /// Optional map of propreties
    pub properties: HashMap<Ident, ValueType>,
}

/// A node that can be contained within an element.
///
/// This is either another element or raw text.
#[derive(Debug)]
pub enum Node {
    /// A sub element
    Element(Element),
    /// Text within an element
    ///
    /// Position is the position of the text within
    /// the source (used for debugging)
    Text(String, Position),
}

/// Contains a value and debugging information
/// for the value.
#[derive(Debug)]
pub struct ValueType {
    /// The parsed value
    pub value: Value,
    /// The position of the value within the source.
    ///
    /// Used for debugging.
    pub position: Position,
}

/// A parsed value for a property
#[derive(Debug)]
pub enum Value {
    /// A 32 bit integer
    Integer(i32),
    /// A 64 bit float (of the form `0.0`)
    Float(f64),
    /// A quoted string
    String(String),
    /// A variable name
    Variable(Ident),
}

#[derive(Debug)]
pub struct ExprType {
    /// The parsed value
    pub expr: Expr,
    /// The position of the value within the source.
    ///
    /// Used for debugging.
    pub position: Position,
}

#[derive(Debug)]
pub enum Expr {
    Value(Value),
    Neg(Box<ExprType>),
    Add(Box<ExprType>, Box<ExprType>),
    Sub(Box<ExprType>, Box<ExprType>),
    Mul(Box<ExprType>, Box<ExprType>),
    Div(Box<ExprType>, Box<ExprType>),
    Call(Ident, Vec<ExprType>),
}

fn parse_document<I>(input: I) -> ParseResult<Document, I>
    where I: Stream<Item=char, Position=SourcePosition>,
          I: Debug,
          I::Range: Debug
{
    spaces()
        .with(many1(try(parser(parse_rule)))) // TODO: Try sucks here
        .map(|e| Document{rules: e})
        .parse_stream(input)
}

fn parse_rule<I>(input: I) -> ParseResult<Rule, I>
    where I: Stream<Item=char, Position=SourcePosition>,
          I: Debug,
          I::Range: Debug
{
    let comments = skip_many(parser(skip_comment));

    let matcher = try(spaces().with(string("@text").map(|_| Matcher::Text)))
        .or(parser(parse_element).map(|v| Matcher::Element(v)));

    let rule = (
        sep_by1(try(matcher), try(spaces().with(token('>')))),
        spaces().with(parser(styles)),
    );

    spaces()
        .with(comments)
        .with(rule)
        .map(|v| Rule {
            matchers: v.0,
            styles: v.1,
        })
        .parse_stream(input)
}

fn parse_element<I>(input: I) -> ParseResult<Element, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    let comments = skip_many(parser(skip_comment));

    let element = (
        parser(ident).skip(look_ahead(
            char('{')
                .or(char('('))
                .or(space())
                .map(|_| ())
        )),
        spaces().with(optional(parser(properties))),
    );

    spaces()
        .with(comments)
        .with(element)
        .map(|v| Element{
            name: v.0,
            properties: v.1.unwrap_or_default(),
        })
        .parse_stream(input)
}

fn ident<I>(input: I) -> ParseResult<Ident, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    (position(), many1(alpha_num().or(char('_'))))
        .map(|(pos, name): (_, String)| Ident {
            name: name,
            position: SourcePosition::into(pos),
        })
        .parse_stream(input)
}

fn styles<I>(input: I) -> ParseResult<HashMap<Ident, ExprType>, I>
    where I: Stream<Item=char, Position=SourcePosition>,
          I: Debug,
          I::Range: Debug
{

    let (_, mut input) = try!(char('{').parse_lazy(input).into());

    let mut styles = HashMap::new();
    loop {
        match input.clone().combine(|input| spaces().with(char('}')).parse_lazy(input).into()) {
            Ok(i) => {
                input = i.1;
                break;
            },
            Err(_) => {}
        };

        match input.clone().combine(|input| spaces().with(parser(skip_comment)).parse_lazy(input).into()) {
            Ok(i) => {
                input = i.1;
                continue;
            },
            Err(_) => {}
        };

        let prop = (
            parser(style_property),
            optional(token(',')),
        );

        let ((prop, end), i) = try!(input.combine(|input| spaces()
                .with(skip_many(parser(skip_comment)))
                .with(prop)
                .parse_lazy(input)
                .into()));
        input = i;
        styles.insert(prop.0, prop.1);

        if end.is_none() {
            let (_, i) = input.clone().combine(|input| spaces().with(char('}')).parse_lazy(input).into())?;
            input = i;
            break;
        }
    }
    Ok((styles, input))
}

fn style_property<I>(input: I) -> ParseResult<(Ident, ExprType), I>
    where I: Stream<Item=char, Position=SourcePosition>,
          I: Debug,
          I::Range: Debug
{;

    let prop = (
        spaces().with(parser(ident)),
        spaces().with(token('=')),
        spaces().with(parser(expr)),
    );
    prop
        .map(|v| (v.0, v.2))
        .parse_stream(input)
}

fn op_prio(c: char) -> u8 {
    match c {
        '-' => 11,
        '+' => 10,
        '/' => 5,
        '*' => 4,
        _ => 255,
    }
}

fn expr<I>(input: I) -> ParseResult<ExprType, I>
    where I: Stream<Item=char, Position=SourcePosition>,
          I: Debug,
          I::Range: Debug
{
    expr_inner(input, 255)
}

fn expr_inner<I>(input: I, max: u8) -> ParseResult<ExprType, I>
    where I: Stream<Item=char, Position=SourcePosition>,
          I: Debug,
          I::Range: Debug
{

    let (neg, mut input) = try!(
        optional((position(), token('-')))
            .parse_lazy(input).into()
    );

    let op_ex = choice!(
        token('+'),
        token('*'),
        token('-'),
        token('/')
    );

    let (bracket, i) = try!(input.combine(|input|
        optional(token('('))
            .parse_lazy(input).into()
    ));
    input = i;

    let v = if bracket.is_some() {
        let (v, i) = try!(input.combine(|input| {
            (parser(|input| expr_inner(input, 255)), token(')'))
                .map(|v| v.0)
                .parse_lazy(input).into()
        }));
        input = i;
        v
    } else {
        let (call, i) = try!(input.combine(|input|
            optional(try((position(), parser(ident), token('('))))
                .parse_lazy(input).into()));
        input = i;

        if let Some((pos, call, _)) = call {
            let (args, i) = try!(input.combine(|input|
                (sep_end_by(
                    spaces().with(parser(expr)),
                    spaces().with(token(','))
                ), spaces().with(token(')')))
                .map(|v| v.0)
                .parse_lazy(input).into()
            ));
            input = i;
            ExprType {
                expr: Expr::Call(call, args),
                position: pos.into(),
            }
        } else {
            let val = parser(value);

            let (v, i) = try!(input.combine(|input| (
                (position(), val)
            ).parse_lazy(input).into()));
            input = i;

            ExprType {
                expr: Expr::Value(v.1.value),
                position: v.0.into(),
            }
        }
    };
    let mut v = if let Some((pos, _)) = neg {
        ExprType {
            expr: Expr::Neg(Box::new(v)),
            position: pos.into(),
        }
    } else {
        v
    };

    loop {
        let op_ex = op_ex.clone();
        let (op, i) = try!(input.combine(|input|
            look_ahead(optional(spaces()
                .with(op_ex.clone())))
            .parse_lazy(input).into()
        ));
        input = i;
        if let Some(op) = op {
            let p = op_prio(op);
            if p > max {
                break;
            }
            let ((pos, op), i) = try!(input.combine(|input| spaces().with((position(), op_ex)).parse_lazy(input).into()));
            input = i;
            let (right, i) = try!(input.combine(|input| spaces()
                .with(parser(|i| expr_inner(i, p)))
                .parse_lazy(input)
                .into()
            ));
            input = i;
            v = ExprType {
                expr: match op {
                    '+' => Expr::Add(Box::new(v), Box::new(right)),
                    '-' => Expr::Sub(Box::new(v), Box::new(right)),
                    '*' => Expr::Mul(Box::new(v), Box::new(right)),
                    '/' => Expr::Div(Box::new(v), Box::new(right)),
                    _ => unreachable!(),
                },
                position: pos.into(),
            };
        } else {
            break;
        }
    }
    Ok((v, input))
}

fn properties<I>(input: I) -> ParseResult<HashMap<Ident, ValueType>, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    let properties = (
        token('('),
        sep_end_by(parser(property), token(',')),
        spaces().with(token(')'))
    );
    properties
        .map(|(_, l, _)| l)
        .parse_stream(input)
}

fn property<I>(input: I) -> ParseResult<(Ident, ValueType), I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    let prop = (
        spaces().with(parser(ident)),
        spaces().with(token('=')),
        spaces().with(parser(value)),
    );
    prop
        .map(|v| (v.0, v.2))
        .parse_stream(input)
}

fn value<I>(input: I) -> ParseResult<ValueType, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    let float = parser(parse_float)
        .map(|v| Value::Float(v));
    let integer = parser(parse_integer)
        .map(|v| Value::Integer(v));

    let string = parser(parse_string)
        .map(|v| Value::String(v));

    let variable = parser(ident)
        .map(|v| Value::Variable(v));

    (
        position(),
        try(float)
            .or(try(integer))
            .or(try(string))
            .or(variable)
    )
        .map(|v| ValueType {
            value: v.1,
            position: SourcePosition::into(v.0),
        })
        .parse_stream(input)
}

fn parse_float<I>(input: I) -> ParseResult<f64, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    let mut buf = String::new();

    let (sign, input) = try!(optional(char('-')).parse_lazy(input).into());
    if let Some(s) = sign {
        buf.push(s);
    }

    let (val, input): (String, _) = try!(input.combine(|input| many1(digit()).parse_lazy(input).into()));
    buf.push_str(&val);
    let (val, input): (String, _) = try!(input.combine(|input|
        char('.')
            .with(many1(digit()))
            .parse_lazy(input).into()
    ));
    buf.push('.');
    buf.push_str(&val);

    let val: f64 = match buf.parse() {
        Ok(val) => val,
        Err(err) => return Err(input.map(|input| ParseError::new(input.position(), Error::Other(err.into())))),
    };

    Ok((val, input))
}

fn parse_integer<I>(input: I) -> ParseResult<i32, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    let mut buf = String::new();

    let (sign, input) = try!(optional(char('-')).parse_lazy(input).into());
    if let Some(s) = sign {
        buf.push(s);
    }

    let (val, input): (String, _) = try!(input.combine(|input| many1(digit()).parse_lazy(input).into()));
    buf.push_str(&val);

    let val: i32 = match buf.parse() {
        Ok(val) => val,
        Err(err) => return Err(input.map(|input| ParseError::new(input.position(), Error::Other(err.into())))),
    };

    Ok((val, input))
}

fn parse_string<I>(input: I) -> ParseResult<String, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    (
        token('"'),
        many(
            try(string(r#"\""#).map(|_| '"'))
                .or(try(string(r#"\t"#).map(|_| '\t')))
                .or(try(string(r#"\n"#).map(|_| '\n')))
                .or(try(string(r#"\r"#).map(|_| '\r')))
                .or(try(string(r#"\\"#).map(|_| '\\')))
                .or(satisfy(|c| c != '"'))
        ),
        token('"'),
    )
        .map(|v| v.1)
        .parse_stream(input)
}

fn skip_comment<I>(input: I) -> ParseResult<(), I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    string("//")
        .with(skip_many(satisfy(|c| c != '\n')))
        .with(spaces())
        .map(|_| ())
        .parse_stream(input)
}

#[cfg(test)]
mod tests {
    use ::format_parse_error;
    use super::*;
    #[test]
    fn test() {
        let source = r##"
// Comments (only single line)
root > panel > image(width=width, height=height) {
    width = width,
    height = height,
    test_expr = width + 6,
    test_expr2 = -5 + -3,
    test_expr3 = height - 6,
    test_expr4 = -3--4,
    test_expr5 = 6 * 3,

    p_test = 5 * (1 + 2) - 3/5,

    call_test = do_thing(5, 3, 4 * 7) / pi(),
    hard_test = -banana() / -(5--4),
}
emoji(type="smile") {
    image = "icons/smile.png",
}

panel > @text {
    color = "#0050AA",
}
        "##;
        let doc = Document::parse(source);
        if let Err(err) = doc {
            println!("");
            format_parse_error(::std::io::stdout(), source.lines(), err).unwrap();
            panic!("^^");
        }
    }
}
