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

use fnv::FnvHashMap;
use common::*;

use combine::*;
use combine::parser::char::*;
use combine::error::*;
use combine::Stream;
use combine::easy::{ParseError,};
use combine::stream::state::{State, SourcePosition};
use super::{Ident, Position};
use std::fmt::Debug;

/// A UI style document
#[derive(Debug)]
pub struct Document<'a> {
    /// A list of rules in this document
    pub rules: Vec<Rule<'a>>,
}

impl <'a> Document<'a> {
    /// Attempts to parse the given string as a document.
    ///
    /// This fails when a syntax error occurs. The returned
    /// error can be formatted in a user friendly format
    /// via the [`format_parse_error`] method.
    ///
    /// # Example
    ///
    /// ```
    /// # use fungui_syntax::style::Document;
    /// assert!(Document::parse(r##"
    /// panel {
    ///     background = "#ff0000",
    /// }
    /// "##).is_ok());
    /// ```
    ///
    /// [`format_parse_error`]: ../fn.format_parse_error.html
    pub fn parse(source: &str) -> Result<Document, ParseError<State<&str, SourcePosition>>> {
        let (doc, _) = parse_document().easy_parse(State::new(source))?;
        Ok(doc)
    }
}

#[derive(Debug, Clone)]
pub struct Rule<'a> {
    pub matchers: Vec<(Matcher<'a>, FnvHashMap<Ident<'a>, ValueType<'a>>)>,
    pub styles: FnvHashMap<Ident<'a>, ExprType<'a>>,
}

#[derive(Debug, Clone)]
pub enum Matcher<'a> {
    Element(Element<'a>),
    Text,
}

/// An element which can contain other elements and/or
/// have properties attached.
///
/// An element does nothing by itself (bar special elements
/// as defined by the program, widgets) and must be controlled
/// via a style document.
#[derive(Debug, Clone)]
pub struct Element<'a> {
    /// The name of this element
    pub name: Ident<'a>,
}

/// Contains a value and debugging information
/// for the value.
#[derive(Debug, Clone)]
pub struct ValueType<'a> {
    /// The parsed value
    pub value: Value<'a>,
    /// The position of the value within the source.
    ///
    /// Used for debugging.
    pub position: Position,
}

/// A parsed value for a property
#[derive(Debug, Clone)]
pub enum Value<'a> {
    /// A boolean value
    Boolean(bool),
    /// A 32 bit integer
    Integer(i32),
    /// A 64 bit float (of the form `0.0`)
    Float(f64),
    /// A quoted string
    String(&'a str),
    /// A variable name
    Variable(Ident<'a>),
}

#[derive(Debug, Clone)]
pub struct ExprType<'a> {
    /// The parsed value
    pub expr: Expr<'a>,
    /// The position of the value within the source.
    ///
    /// Used for debugging.
    pub position: Position,
}

#[derive(Debug, Clone)]
pub enum Expr<'a> {
    Value(Value<'a>),
    Neg(Box<ExprType<'a>>),

    Not(Box<ExprType<'a>>),
    And(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Or(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Xor(Box<ExprType<'a>>, Box<ExprType<'a>>),

    Add(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Sub(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Mul(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Div(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Rem(Box<ExprType<'a>>, Box<ExprType<'a>>),

    Equal(Box<ExprType<'a>>, Box<ExprType<'a>>),
    NotEqual(Box<ExprType<'a>>, Box<ExprType<'a>>),
    LessEqual(Box<ExprType<'a>>, Box<ExprType<'a>>),
    GreaterEqual(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Less(Box<ExprType<'a>>, Box<ExprType<'a>>),
    Greater(Box<ExprType<'a>>, Box<ExprType<'a>>),

    IntToFloat(Box<ExprType<'a>>),
    FloatToInt(Box<ExprType<'a>>),

    Call(Ident<'a>, Vec<ExprType<'a>>),
}

fn parse_document<'a, I>() -> impl Parser<Input = I, Output = Document<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let rule = (parse_rule(), spaces()).map(|v| v.0);
    spaces()
        .with(many1(rule))
        .map(|e| Document { rules: e })
}

fn parse_rule<'a, I>() -> impl Parser<Input = I, Output = Rule<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let comments = skip_many(skip_comment());

    let matcher = (
        try(spaces().with(string("@text").map(|_| Matcher::Text)))
            .or(parse_element().map(|v| Matcher::Element(v))),
        optional(properties()).map(|v| v.unwrap_or_default()),
    );

    let rule = (
        sep_by1(try(matcher), try(spaces().with(token('>')))),
        spaces().with(parser(styles)),
    );

    spaces()
        .with(comments)
        .with(rule)
        .map(|v| {
            Rule {
                matchers: v.0,
                styles: v.1,
            }
        })
}

fn parse_element<'a, I>() -> impl Parser<Input = I, Output = Element<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let comments = skip_many(skip_comment());

    let element = ident().skip(look_ahead(char('{').or(char('(')).or(space()).map(|_| ())));

    spaces()
        .with(comments)
        .with(element)
        .map(|v| Element { name: v })
}

fn styles<'a, I>(input: &mut I) -> ParseResult<FnvHashMap<Ident<'a>, ExprType<'a>>, I>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let (_, _) = char('{').parse_stream(input)?;

    enum Flow<T> {
        Continue(T),
        Break,
    }

    let mut styles = FnvHashMap::default();
    loop {
        let prop = (style_property(), optional(token(',')));
        let (ret, _) = spaces()
                .with(skip_many(skip_comment()))
                .with(
                    try(char('}').map(|_| Flow::Break))
                        .or(
                            prop
                            .map(|v| Flow::Continue(v.0))
                        ),
                )
                .parse_stream(input)?;
        if let Flow::Continue(s) = ret {
            styles.insert(s.0, s.1);
        } else {
            break;
        }
    }
    Ok((styles, Consumed::Consumed(())))
}

fn style_property<'a, I>() -> impl Parser<Input = I, Output = (Ident<'a>, ExprType<'a>)>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    (
        spaces().with(ident()),
        spaces().with(token('=')),
        spaces().with(parser(expr)),
    ).map(|v| (v.0, v.2))
}

fn expr<'a, I>(input: &mut I) -> ParseResult<ExprType<'a>, I>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let skip_spaces = || spaces().silent();

    let (mut current, _) =
        skip_spaces()
        .with(parser(bool_ops))
        .skip(skip_spaces())
        .parse_stream(input)?;

    loop {
        let (op, _) = match (position(), choice((
                attempt(string("==")),
                attempt(string("!=")),
                attempt(string("<=")),
                attempt(string(">=")),
                string("<"),
                string(">"),
            )))
            .skip(skip_spaces())
            .parse_stream(input)
        {
            Ok(v) => v,
            Err(_) => break,
        };
        let (other, _) = parser(bool_ops)
            .skip(skip_spaces())
            .parse_stream(input)?;
        current = ExprType {
            position: SourcePosition::into(op.0),
            expr: match op.1 {
                "==" => Expr::Equal(Box::new(current), Box::new(other)),
                "!=" => Expr::NotEqual(Box::new(current), Box::new(other)),
                "<=" => Expr::LessEqual(Box::new(current), Box::new(other)),
                ">=" => Expr::GreaterEqual(Box::new(current), Box::new(other)),
                "<" => Expr::Less(Box::new(current), Box::new(other)),
                ">" => Expr::Greater(Box::new(current), Box::new(other)),
                _ => unreachable!(),
            },
        };
    }
    Ok((current, Consumed::Consumed(())))
}

fn bool_ops<'a, I>(input: &mut I) -> ParseResult<ExprType<'a>, I>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let skip_spaces = || spaces().silent();

    let (mut current, _) = parser(term1)
        .skip(skip_spaces())
        .parse_stream(input)?;

    loop {
        let (op, _) = match (position(), choice((
                attempt(string("&&")),
                attempt(string("||")),
                string("^"),
            )))
            .skip(skip_spaces())
            .parse_stream(input)
        {
            Ok(v) => v,
            Err(_) => break,
        };
        let (other, _) = parser(term1)
            .skip(skip_spaces())
            .parse_stream(input)?;
        current = ExprType {
            position: SourcePosition::into(op.0),
            expr: match op.1 {
                "&&" => Expr::And(Box::new(current), Box::new(other)),
                "||" => Expr::Or(Box::new(current), Box::new(other)),
                "^" => Expr::Xor(Box::new(current), Box::new(other)),
                _ => unreachable!(),
            },
        };
    }

    Ok((current, Consumed::Consumed(())))
}

fn term1<'a, I>(input: &mut I) -> ParseResult<ExprType<'a>, I>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let skip_spaces = || spaces().silent();

    let (mut current, _) = parser(term2)
        .skip(skip_spaces())
        .parse_stream(input)?;

    loop {
        let (op, _) = match (position(), choice((char('+'), char('-'))))
            .skip(skip_spaces())
            .parse_stream(input)
        {
            Ok(v) => v,
            Err(_) => break,
        };
        let (other, _) = parser(term2)
            .skip(skip_spaces())
            .parse_stream(input)?;
        current = ExprType {
            position: SourcePosition::into(op.0),
            expr: match op.1 {
                '+' => Expr::Add(Box::new(current), Box::new(other)),
                '-' => Expr::Sub(Box::new(current), Box::new(other)),
                _ => unreachable!(),
            },
        };
    }

    Ok((current, Consumed::Consumed(())))
}

fn term2<'a, I>(input: &mut I) -> ParseResult<ExprType<'a>, I>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let skip_spaces = || spaces().silent();

    let (mut current, _) = factor()
        .skip(skip_spaces())
        .parse_stream(input)?;

    loop {
        let (op, _) = match (position(), choice((char('*'), char('/'), char('%'))))
            .skip(skip_spaces())
            .parse_stream(input)
        {
            Ok(v) => v,
            Err(_) => break,
        };
        let (other, _) = factor()
            .skip(skip_spaces())
            .parse_stream(input)?;
        current = ExprType {
            position: SourcePosition::into(op.0),
            expr: match op.1 {
                '*' => Expr::Mul(Box::new(current), Box::new(other)),
                '/' => Expr::Div(Box::new(current), Box::new(other)),
                '%' => Expr::Rem(Box::new(current), Box::new(other)),
                _ => unreachable!(),
            },
        };
    }
    Ok((current, Consumed::Consumed(())))
}

fn factor<'a, I>() -> impl Parser<Input = I, Output = ExprType<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let skip_spaces = || spaces().silent();

    let brackets = char('(')
        .skip(skip_spaces())
        .with(parser(expr))
        .skip(skip_spaces())
        .skip(char(')'));


    let call = (ident(), char('(')
        .skip(skip_spaces())
        .with(sep_end_by(parser(expr).skip(skip_spaces()), char(',')))
        .skip(skip_spaces())
        .skip(char(')'))
    ).map(|v| Expr::Call(v.0, v.1));

    let float_to_int = string("int")
        .expected("int cast")
        .skip(string("("))
        .skip(skip_spaces())
        .with(parser(expr))
        .map(|v| Expr::FloatToInt(Box::new(v)))
        .skip(skip_spaces())
        .skip(char(')'));
    let int_to_float = string("float")
        .expected("float cast")
        .skip(string("("))
        .skip(skip_spaces())
        .with(parser(expr))
        .map(|v| Expr::IntToFloat(Box::new(v)))
        .skip(skip_spaces())
        .skip(char(')'));

    let not = char('!')
        .skip(skip_spaces())
        .with(parser(expr))
        .map(|v| Expr::Not(Box::new(v)));

    let neg = char('-')
        .skip(skip_spaces())
        .with(parser(expr))
        .map(|v| Expr::Neg(Box::new(v)));

    (
        position(),
        choice((
            attempt(float_to_int),
            attempt(int_to_float),
            attempt(brackets.map(|v| v.expr)),
            attempt(call),
            attempt(value().map(|v| Expr::Value(v.value))),
            attempt(not),
            attempt(neg),
        ))
    ).map(|v| ExprType {
        position: SourcePosition::into(v.0),
        expr: v.1,
    })
}

fn properties<'a, I>() -> impl Parser<Input = I, Output = FnvHashMap<Ident<'a>, ValueType<'a>>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    (
        token('('),
        sep_end_by(property(), token(',')),
        spaces().with(token(')')),
    ).map(|(_, l, _)| l)
}

fn property<'a, I>() -> impl Parser<Input = I, Output = (Ident<'a>, ValueType<'a>)>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    (
        spaces().with(ident()),
        spaces().with(token('=')),
        spaces().with(value()),
    ).map(|v| (v.0, v.2))
}

fn value<'a, I>() -> impl Parser<Input = I, Output = ValueType<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let boolean = parse_bool().map(|v| Value::Boolean(v));
    let float = parse_float().map(|v| Value::Float(v));
    let integer = parse_integer().map(|v| Value::Integer(v));

    let string = parse_string().map(|v| Value::String(v));

    let variable = ident().map(|v| Value::Variable(v));

    (
        position(),
        try(boolean)
            .or(try(float))
            .or(try(integer))
            .or(try(variable))
            .or(string),
    ).map(|v| {
            ValueType {
                value: v.1,
                position: SourcePosition::into(v.0),
            }
        })
}

#[cfg(test)]
mod tests {
    use format_parse_error;
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
