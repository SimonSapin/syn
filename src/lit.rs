/// Literal kind.
///
/// E.g. `"foo"`, `42`, `12.34` or `bool`
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Lit {
    /// A string literal (`"foo"`)
    Str(String, StrStyle),
    /// A byte string (`b"foo"`)
    ByteStr(Vec<u8>),
    /// A byte char (`b'f'`)
    Byte(u8),
    /// A character literal (`'a'`)
    Char(char),
    /// An integer literal (`1`)
    Int(u64, IntTy),
    /// A float literal (`1f64` or `1E10f64` or `1.0E10`)
    Float(String, FloatTy),
    /// A boolean literal
    Bool(bool),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum StrStyle {
    /// A regular string, like `"foo"`
    Cooked,
    /// A raw string, like `r##"foo"##`
    ///
    /// The uint is the number of `#` symbols used
    Raw(usize),
}

impl From<String> for Lit {
    fn from(input: String) -> Lit {
        Lit::Str(input, StrStyle::Cooked)
    }
}

impl<'a> From<&'a str> for Lit {
    fn from(input: &str) -> Lit {
        Lit::Str(input.into(), StrStyle::Cooked)
    }
}

impl From<Vec<u8>> for Lit {
    fn from(input: Vec<u8>) -> Lit {
        Lit::ByteStr(input)
    }
}

impl<'a> From<&'a [u8]> for Lit {
    fn from(input: &[u8]) -> Lit {
        Lit::ByteStr(input.into())
    }
}

impl From<char> for Lit {
    fn from(input: char) -> Lit {
        Lit::Char(input)
    }
}

impl From<bool> for Lit {
    fn from(input: bool) -> Lit {
        Lit::Bool(input)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum IntTy {
    Isize,
    I8,
    I16,
    I32,
    I64,
    Usize,
    U8,
    U16,
    U32,
    U64,
    Unsuffixed,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FloatTy {
    F32,
    F64,
    Unsuffixed,
}

macro_rules! impl_from_for_lit {
    (Int, [$($rust_type:ty => $syn_type:expr),+]) => {
        $(
            impl From<$rust_type> for Lit {
                fn from(input: $rust_type) -> Lit {
                    Lit::Int(input as u64, $syn_type)
                }
            }
        )+
    };
    (Float, [$($rust_type:ty => $syn_type:expr),+]) => {
        $(
            impl From<$rust_type> for Lit {
                fn from(input: $rust_type) -> Lit {
                    Lit::Float(format!("{}", input), $syn_type)
                }
            }
        )+
    };
}

impl_from_for_lit! {Int, [
    isize => IntTy::Isize,
    i8 => IntTy::I8,
    i16 => IntTy::I16,
    i32 => IntTy::I32,
    i64 => IntTy::I64,
    usize => IntTy::Usize,
    u8 => IntTy::U8,
    u16 => IntTy::U16,
    u32 => IntTy::U32,
    u64 => IntTy::U64
]}

impl_from_for_lit! {Float, [
    f32 => FloatTy::F32,
    f64 => FloatTy::F64
]}

#[cfg(feature = "parsing")]
pub mod parsing {
    use super::*;
    use escape::{cooked_char, cooked_string, raw_string};
    use space::whitespace;
    use nom::IResult;

    named!(pub lit -> Lit, alt!(
        string
        |
        byte_string
        |
        byte
        |
        character
        |
        int => { |(value, ty)| Lit::Int(value, ty) }
    // TODO: Float
        |
        keyword!("true") => { |_| Lit::Bool(true) }
        |
        keyword!("false") => { |_| Lit::Bool(false) }
    ));

    named!(string -> Lit, alt!(
        quoted_string => { |s| Lit::Str(s, StrStyle::Cooked) }
        |
        preceded!(
            punct!("r"),
            raw_string
        ) => { |(s, n)| Lit::Str(s, StrStyle::Raw(n)) }
    ));

    named!(pub quoted_string -> String, delimited!(
        punct!("\""),
        cooked_string,
        tag!("\"")
    ));

    named!(byte_string -> Lit, alt!(
        delimited!(
            punct!("b\""),
            cooked_string,
            tag!("\"")
        ) => { |s: String| Lit::ByteStr(s.into_bytes()) }
        |
        preceded!(
            punct!("br"),
            raw_string
        ) => { |(s, _): (String, _)| Lit::ByteStr(s.into_bytes()) }
    ));

    named!(byte -> Lit, do_parse!(
        punct!("b") >>
        tag!("'") >>
        ch: cooked_char >>
        tag!("'") >>
        (Lit::Byte(ch as u8))
    ));

    named!(character -> Lit, do_parse!(
        punct!("'") >>
        ch: cooked_char >>
        tag!("'") >>
        (Lit::Char(ch))
    ));

    named!(pub int -> (u64, IntTy), tuple!(
        preceded!(
            option!(whitespace),
            digits
        ),
        alt!(
            tag!("isize") => { |_| IntTy::Isize }
            |
            tag!("i8") => { |_| IntTy::I8 }
            |
            tag!("i16") => { |_| IntTy::I16 }
            |
            tag!("i32") => { |_| IntTy::I32 }
            |
            tag!("i64") => { |_| IntTy::I64 }
            |
            tag!("usize") => { |_| IntTy::Usize }
            |
            tag!("u8") => { |_| IntTy::U8 }
            |
            tag!("u16") => { |_| IntTy::U16 }
            |
            tag!("u32") => { |_| IntTy::U32 }
            |
            tag!("u64") => { |_| IntTy::U64 }
            |
            epsilon!() => { |_| IntTy::Unsuffixed }
        )
    ));

    pub fn digits(input: &str) -> IResult<&str, u64> {
        let mut value = 0u64;
        let mut len = 0;
        let mut bytes = input.bytes().peekable();
        while let Some(&b) = bytes.peek() {
            match b {
                b'0'...b'9' => {
                    value = match value.checked_mul(10) {
                        Some(value) => value,
                        None => return IResult::Error,
                    };
                    value = match value.checked_add((b - b'0') as u64) {
                        Some(value) => value,
                        None => return IResult::Error,
                    };
                    bytes.next();
                    len += 1;
                }
                _ => break,
            }
        }
        if len > 0 {
            IResult::Done(&input[len..], value)
        } else {
            IResult::Error
        }
    }
}

#[cfg(feature = "printing")]
mod printing {
    use super::*;
    use quote::{Tokens, ToTokens};
    use std::{ascii, iter};
    use std::fmt::{self, Display};

    impl ToTokens for Lit {
        fn to_tokens(&self, tokens: &mut Tokens) {
            match *self {
                Lit::Str(ref s, StrStyle::Cooked) => s.to_tokens(tokens),
                Lit::Str(ref s, StrStyle::Raw(n)) => {
                    tokens.append(&format!("r{delim}\"{string}\"{delim}",
                        delim = iter::repeat("#").take(n).collect::<String>(),
                        string = s));
                }
                Lit::ByteStr(ref v) => {
                    let mut escaped = "b\"".to_string();
                    for &ch in v.iter() {
                        escaped.extend(ascii::escape_default(ch).map(|c| c as char));
                    }
                    escaped.push('"');
                    tokens.append(&escaped);
                }
                Lit::Byte(b) => tokens.append(&format!("b{:?}", b as char)),
                Lit::Char(ch) => ch.to_tokens(tokens),
                Lit::Int(value, ty) => tokens.append(&format!("{}{}", value, ty)),
                Lit::Float(ref value, ty) => tokens.append(&format!("{}{}", value, ty)),
                Lit::Bool(true) => tokens.append("true"),
                Lit::Bool(false) => tokens.append("false"),
            }
        }
    }

    impl Display for IntTy {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            match *self {
                IntTy::Isize => formatter.write_str("isize"),
                IntTy::I8 => formatter.write_str("i8"),
                IntTy::I16 => formatter.write_str("i16"),
                IntTy::I32 => formatter.write_str("i32"),
                IntTy::I64 => formatter.write_str("i64"),
                IntTy::Usize => formatter.write_str("usize"),
                IntTy::U8 => formatter.write_str("u8"),
                IntTy::U16 => formatter.write_str("u16"),
                IntTy::U32 => formatter.write_str("u32"),
                IntTy::U64 => formatter.write_str("u64"),
                IntTy::Unsuffixed => Ok(()),
            }
        }
    }

    impl Display for FloatTy {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            match *self {
                FloatTy::F32 => formatter.write_str("f32"),
                FloatTy::F64 => formatter.write_str("f64"),
                FloatTy::Unsuffixed => Ok(()),
            }
        }
    }
}
