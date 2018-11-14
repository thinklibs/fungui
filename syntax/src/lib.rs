
extern crate combine;
extern crate fnv;

pub mod desc;
pub mod style;
pub(crate) mod common;


use combine::easy::{ParseError};
use combine::stream::state::{State, SourcePosition};
use std::io::{self, Write};
use std::hash::{Hash, Hasher};
use std::fmt::{self, Display, Formatter};

pub type PError<'a> = ParseError<State<&'a str, SourcePosition>>;

pub use combine::easy::{Errors, Error, Info};

/// An identifier.
///
/// An identifier is made up of either letters, numbers
/// or `_`.
#[derive(Debug, Default, Clone)]
pub struct Ident<'a> {
    /// The identifier's value/name
    pub name: &'a str,
    /// The position of the identifier within the source.
    ///
    /// Used for debugging.
    pub position: Position,
}

impl <'a> PartialEq for Ident<'a> {
    fn eq(&self, o: &Ident) -> bool {
        self.name == o.name
    }
}

impl <'a> Eq for Ident<'a> {}

impl <'a> Hash for Ident<'a> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.name.hash(state);
    }
}

/// The position in the source file where the
/// the ident/value/etc was defined.
///
/// This is used to provide better debugging support
/// when an error in encounted.
#[derive(Clone, Copy, Debug, Default)]
pub struct Position {
    /// The line this relates to.
    ///
    /// This starts at line 1 (not 0)
    pub line_number: i32,
    /// The column this relates to.
    ///
    /// This starts at line 1 (not 0)
    pub column: i32,
}

impl From<SourcePosition> for Position {
    fn from(v: SourcePosition) -> Position {
        Position {
            line_number: v.line,
            column: v.column,
        }
    }
}

impl From<Position> for SourcePosition {
    fn from(v: Position) -> SourcePosition {
        SourcePosition {
            line: v.line_number,
            column: v.column,
        }
    }
}

impl Display for Position {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "{}:{}", self.line_number, self.column)
    }
}

/// Formats the error in a user friendly format
pub fn format_error<'a, I, W>(
    mut w: W,
    source: I,
    pos: Position,
    len: usize,
    msg: &str,
    label: &str,
) -> io::Result<()>
where
    W: Write,
    I: Iterator<Item = &'a str>,
{
    use std::cmp::max;
    let number_len = (pos.line_number + 1).to_string().len();
    write!(&mut w, "error: {}\n", msg)?;
    write!(
        &mut w,
        "{:width$}--> {}:{}\n",
        "",
        pos.line_number,
        pos.column,
        width = number_len,
    )?;
    let skip = max(0, pos.line_number - 2) as usize;
    let take = if pos.line_number == 1 {
        write!(&mut w, "{:width$} |\n", "", width = number_len)?;
        2
    } else {
        3
    };

    for (no, line) in source.enumerate().skip(skip).take(take) {
        let target_line = no == (pos.line_number - 1) as usize;
        if target_line {
            write!(&mut w, "{:width$} | {}\n", no + 1, line, width = number_len)?;
            write!(
                &mut w,
                "{:width$} | {:offset$}{:^<len$} {}\n",
                "",
                "",
                "",
                label,
                width = number_len,
                offset = pos.column as usize - 1,
                len = len
            )?;
        } else {
            write!(&mut w, "{:width$} | {}\n", "", line, width = number_len)?;
        }
    }
    Ok(())
}

/// Formats a parsing error using [`format_error`].
///
/// [`format_error`]: fn.format_error.html
pub fn format_parse_error<'a, I, W>(
    w: W,
    source: I,
    err: ParseError<State<&'a str, SourcePosition>>,
) -> Result<(), Box<::std::error::Error>>
where
    W: Write,
    I: Iterator<Item = &'a str>,
{
    use combine::easy::{Error, Info};
    use std::fmt::Write;
    let mut msg = String::new();
    let mut label = String::new();
    enum Type {
        Unexpected,
        Message,
        Unknown,
    }
    let ty = if let Some(first) = err.errors.first() {
        match *first {
            Error::Unexpected(..) => Type::Unexpected,
            _ => Type::Message,
        }
    } else {
        Type::Unknown
    };

    let mut token_len = 1;

    match ty {
        Type::Unknown => msg.push_str("Unknown error occurred"),
        Type::Message => {
            let len = err.errors.len();
            for (idx, err) in err.errors.into_iter().enumerate() {
                match err {
                    Error::Message(ref m) => match *m {
                        Info::Owned(ref m) => msg.push_str(m),
                        Info::Borrowed(m) => msg.push_str(m),
                        _ => unimplemented!(),
                    },
                    Error::Other(ref err) => write!(&mut msg, "{}", err)?,
                    Error::Expected(ref t) => write!(&mut msg, "Expected: {}", t)?,
                    Error::Unexpected(ref t) => write!(&mut msg, "Unexpected: {}", t)?,
                }
                if idx != len - 1 {
                    msg.push_str(", ");
                }
            }
        },
        Type::Unexpected => {
            msg.push_str("Unexpected '");
            label.push_str("Unexpected '");
            if let Some(first) = err.errors.first() {
                match *first {
                    Error::Unexpected(ref m) => match *m {
                        Info::Owned(ref m) => {
                            msg.push_str(m);
                            label.push_str(m);
                            token_len = m.len();
                        }
                        Info::Borrowed(m) => {
                            msg.push_str(m);
                            label.push_str(m);
                            token_len = m.len();
                        }
                        Info::Token(t) => {
                            write!(&mut msg, "{}", t)?;
                            write!(&mut label, "{}", t)?;
                        }
                        _ => unimplemented!(),
                    },
                    _ => unimplemented!(),
                }
            }
            label.push_str("'");
            msg.push_str("' expected ");
            if err.errors.len() > 2 {
                msg.push_str("either ");
            }
            let len = err.errors[1..].len() as isize;
            for (i, err) in err.errors[1..].iter().enumerate() {
                msg.push('\'');
                match *err {
                    Error::Expected(ref m) => match *m {
                        Info::Owned(ref m) => {
                            msg.push_str(m);
                        }
                        Info::Borrowed(m) => {
                            msg.push_str(m);
                        }
                        Info::Token(t) => {
                            write!(&mut msg, "{}", t)?;
                        }
                        _ => unimplemented!(),
                    },
                    _ => {}
                }
                msg.push('\'');
                if (i as isize) < len - 2 {
                    msg.push_str(", ");
                } else if i as isize == len - 2 {
                    msg.push_str(" or ");
                }
            }
        }
    }

    format_error(w, source, err.position.into(), token_len, &msg, &label)?;
    Ok(())
}
