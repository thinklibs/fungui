
use combine::*;
use combine::parser::char::*;
use combine::parser::range::*;
use combine::error::*;
use combine::Stream;
use combine::stream::state::SourcePosition;
use combine::stream::StreamErrorFor;
use super::Ident;
use std::fmt::Debug;

pub(crate) fn ident<'a, I>() -> impl Parser<Input = I, Output = Ident<'a>>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    (position(), take_while1(|c: char| c.is_alphanumeric() || c == '_'))
        .map(|(pos, name): (_, &str)| {
            Ident {
                name: name,
                position: SourcePosition::into(pos),
            }
        })
}

pub(crate) fn parse_bool<'a, I>() -> impl Parser<Input = I, Output = bool>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    try(string("true").map(|_| true))
        .or(string("false").map(|_| false))
}

pub(crate) fn parse_float<'a, I>() -> impl Parser<Input = I, Output = f64> + 'a
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    from_str(take_while1(|c: char| c.is_digit(10) || c == '.' || c == '-')
        .and_then(|v: &str| if v.contains('.') {
            Ok(v)
        } else {
            Err(StreamErrorFor::<I>::expected_static_message("float"))
        } ))
}

pub(crate) fn parse_integer<'a, I>() -> impl Parser<Input = I, Output = i32>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    from_str(
        Parser::expected(take_while1(|c: char| c.is_digit(10) || c == '-'), "integer")
    )

}

pub(crate) fn parse_string<'a, I>() -> impl Parser<Input = I, Output = &'a str>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    (
        token('"'),
        recognize(skip_many(
            try(string(r#"\""#).map(|_| '"'))
                .or(try(string(r#"\t"#).map(|_| '\t')))
                .or(try(string(r#"\n"#).map(|_| '\n')))
                .or(try(string(r#"\r"#).map(|_| '\r')))
                .or(try(string(r#"\\"#).map(|_| '\\')))
                .or(satisfy(|c| c != '"')),
        )),
        token('"'),
    ).map(|v| v.1)
}

pub(crate) fn skip_comment<'a, I>() -> impl Parser<Input = I, Output = ()>
    where
        I: Debug + Stream<Item=char, Position=SourcePosition, Range = &'a str> + RangeStream + 'a,
        <I as StreamOnce>::Error: combine::ParseError<I::Item, I::Range, I::Position>,
{
    string("//")
        .with(skip_many(satisfy(|c| c != '\n')))
        .with(spaces())
        .map(|_| ())
}