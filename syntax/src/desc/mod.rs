//! Parser for the UI description format
//!
//! This module contains the AST and parser for the
//! format used to describe the layout of a UI element.
//!
//! The format is as follows:
//!
//! ```text,ignore
//! // Comments (only single line)
//!
//! // Name of an element. Can be made up from any
//! // letter, number or _
//! root {
//!     // Nested elements supported
//!     panel {
//!
//!     }
//!     // Properties can be specified within ()
//!     // as `key=value` pairs
//!     image(src="example.png", width=150, height=150) {
//!
//!     }
//!     // {} is optional
//!     emoji(type="smile")
//!     // As is ()
//!     spacer
//!     // Text can be used as well (quoted)
//!     "Hello world"
//! }
//! ```

use combine::*;
use combine::char::{char, digit, alpha_num, spaces, string, space};
use combine::primitives::{Error, SourcePosition};
use std::collections::HashMap;
use super::{
    Ident,
    Position,
};

/// A UI description document
///
/// Currently a document is made up of a single element.
#[derive(Debug)]
pub struct Document {
    /// The root element of the element
    pub root: Element,
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
    /// # use stylish_syntax::desc::Document;
    /// assert!(Document::parse(r#"
    /// root {
    ///     "hello world"
    /// }
    /// "#).is_ok());
    /// ```
    ///
    /// [`format_parse_error`]: ../fn.format_parse_error.html
    pub fn parse(source: &str) -> Result<Document, ParseError<State<&str>>> {
        let (doc, _) = parser(parse_document).parse(State::new(source))?;
        Ok(doc)
    }
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
    /// Optional list of nodes within this element
    pub nodes: Vec<Node>,
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
    /// A boolean value
    Boolean(bool),
    /// A 32 bit integer
    Integer(i32),
    /// A 64 bit float (of the form `0.0`)
    Float(f64),
    /// A quoted string
    String(String),
}

fn parse_document<I>(input: I) -> ParseResult<Document, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    spaces()
        .with(parser(parse_element))
        .map(|e| Document{root: e})
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
        spaces().with(optional(parser(body))),
    );

    spaces()
        .with(comments)
        .with(element)
        .map(|v| Element{
            name: v.0,
            properties: v.1.unwrap_or_default(),
            nodes: v.2.unwrap_or_default(),
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

fn body<I>(input: I) -> ParseResult<Vec<Node>, I>
    where I: Stream<Item=char, Position=SourcePosition>
{

    let (_, mut input) = try!(char('{').parse_lazy(input).into());

    let mut nodes = Vec::new();
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

        let (node, i) = try!(input.combine(|input| spaces()
                .with(skip_many(parser(skip_comment)))
                .with(
                    (position(), parser(parse_string)).map(|v| Node::Text(v.1, SourcePosition::into(v.0)))
                        .or(parser(parse_element).map(Node::Element))
                )
                .parse_lazy(input)
                .into()));
        input = i;
        nodes.push(node);
    }
    Ok((nodes, input))
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
    let boolean = parser(parse_bool)
        .map(|v| Value::Boolean(v));
    let float = parser(parse_float)
        .map(|v| Value::Float(v));
    let integer = parser(parse_integer)
        .map(|v| Value::Integer(v));

    let string = parser(parse_string)
        .map(|v| Value::String(v));

    (
        position(),
        try(boolean)
            .or(try(float))
            .or(try(integer))
            .or(string)
    )
        .map(|v| ValueType {
            value: v.1,
            position: SourcePosition::into(v.0),
        })
        .parse_stream(input)
}

fn parse_bool<I>(input: I) -> ParseResult<bool, I>
    where I: Stream<Item=char, Position=SourcePosition>
{
    let (t, input) = try!(optional(string("true")).parse_lazy(input).into());
    if t.is_some() {
        return Ok((true, input));
    }
    let (_, input) = try!(input.combine(|input| string("false").parse_lazy(input).into()));
    Ok((false, input))
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
        let source = r#"
// <- comments
// () are optional
root(
    testing=3, hello=4.56,
    test="hello world",
    quoted="this has \"quotes\" and \\",
    negative=-5, negfloat=-634.354
) {
    spacer
    // Names can be anything, same for properties.
    // Styles control most things, a few special
    // widgets tied to certain names. Doesn't
    // matter to the parser though
    panel(width=500, height=300) {
        "Text can be placed within elements"
        emoji(name="smile")
        "and between them"
    }
}
        "#;
        let doc = Document::parse(source);

        if let Err(err) = doc {
            println!("");
            format_parse_error(::std::io::stdout(), source.lines(), err).unwrap();
            panic!("^^");
        }
    }

    #[test]
    fn test_print_invalid_ident() {
        let source = r#"roo$t {

}
        "#;
        let doc = Document::parse(source);
        if let Err(err) = doc {
            let mut out: Vec<u8> = Vec::new();
            format_parse_error(&mut out, source.lines(), err).unwrap();
            assert_eq!(
                String::from_utf8_lossy(&out).lines().map(|v| v.trim_right().to_owned() + "\n").collect::<String>(),
r#"error: Unexpected '$' expected either '{', '(' or 'whitespace'
 --> 1:4
  |
1 | roo$t {
  |    ^ Unexpected '$'
  |
"#);
        } else {
            panic!("Expected error");
        }
    }
}
