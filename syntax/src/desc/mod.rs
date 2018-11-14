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

/// A UI description document
///
/// Currently a document is made up of a single element.
#[derive(Debug)]
pub struct Document<'a> {
    /// The root element of the element
    pub root: Element<'a>,
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
    /// # use fungui_syntax::desc::Document;
    /// assert!(Document::parse(r#"
    /// root {
    ///     "hello world"
    /// }
    /// "#).is_ok());
    /// ```
    ///
    /// [`format_parse_error`]: ../fn.format_parse_error.html
    pub fn parse(source: &str) -> Result<Document, ParseError<State<&str, SourcePosition>>> {
        let (doc, _) = parse_document().easy_parse(State::new(source))?;
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
pub struct Element<'a> {
    /// The name of this element
    pub name: Ident<'a>,
    /// Optional map of propreties
    pub properties: FnvHashMap<Ident<'a>, ValueType<'a>>,
    /// Optional list of nodes within this element
    pub nodes: Vec<Node<'a>>,
}

/// A node that can be contained within an element.
///
/// This is either another element or raw text.
#[derive(Debug)]
pub enum Node<'a> {
    /// A sub element
    Element(Element<'a>),
    /// Text within an element
    ///
    /// Position is the position of the text within
    /// the source (used for debugging)
    Text(&'a str, Position, FnvHashMap<Ident<'a>, ValueType<'a>>),
}

/// Contains a value and debugging information
/// for the value.
#[derive(Debug)]
pub struct ValueType<'a> {
    /// The parsed value
    pub value: Value<'a>,
    /// The position of the value within the source.
    ///
    /// Used for debugging.
    pub position: Position,
}

/// A parsed value for a property
#[derive(Debug)]
pub enum Value<'a> {
    /// A boolean value
    Boolean(bool),
    /// A 32 bit integer
    Integer(i32),
    /// A 64 bit float (of the form `0.0`)
    Float(f64),
    /// A quoted string
    String(&'a str),
}

fn parse_document<'a, I>() -> impl Parser<Input = I, Output = Document<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    spaces()
        .with(parse_element())
        .map(|e| Document { root: e })
}

fn parse_element<'a, I>() -> impl Parser<Input = I, Output = Element<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let comments = skip_many(skip_comment());

    let element = (
        ident().skip(look_ahead(char('{').or(char('(')).or(space()).map(|_| ()))),
        spaces().with(optional(properties())),
        spaces().with(optional(parser(body))),
    );

    spaces()
        .with(comments)
        .with(element)
        .map(|v| {
            Element {
                name: v.0,
                properties: v.1.unwrap_or_default(),
                nodes: v.2.unwrap_or_default(),
            }
        })
}

fn body<'a, I>(input: &mut I) -> ParseResult<Vec<Node<'a>>, I>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    let (_, _) = char('{').parse_stream(input)?;

    enum Flow<T> {
        Continue(T),
        Break,
    }

    let mut nodes = Vec::new();
    loop {
        let (ret, _) = spaces()
                .with(skip_many(skip_comment()))
                .with(
                    try(char('}').map(|_| Flow::Break))
                        .or(
                            (
                                position(),
                                parse_string(),
                                optional(properties()),
                            ).map(|v| {
                                Node::Text(v.1, SourcePosition::into(v.0), v.2.unwrap_or_default())
                            })
                            .or(parse_element().map(Node::Element))
                            .map(|v| Flow::Continue(v))
                        ),
                )
                .parse_stream(input)?;
        if let Flow::Continue(node) = ret {
            nodes.push(node);
        } else {
            break;
        }
    }
    Ok((nodes, Consumed::Consumed(())))
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

    (
        position(),
        try(boolean).or(try(float)).or(try(integer)).or(string),
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
        "and between them"(text_val=4)
    }

    empty_string {
        ""
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
                String::from_utf8_lossy(&out)
                    .lines()
                    .map(|v| v.trim_right().to_owned() + "\n")
                    .collect::<String>(),
                r#"error: Unexpected '$' expected either '{', '(' or 'whitespace'
 --> 1:4
  |
1 | roo$t {
  |    ^ Unexpected '$'
  |
"#
            );
        } else {
            panic!("Expected error");
        }
    }
}
