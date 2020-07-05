// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Minor phases: 1, 2, 5, 6

use std::convert::TryFrom;

use log::{log_enabled, trace};

use crate::front::c::location::Location;
use crate::front::c::message::MessageKind;
use crate::front::c::token::{CharToken, PPToken, PPTokenKind};
use crate::front::c::tuctx::TUCtx;

/// Phase 1: Convert trigraphs
pub fn convert_trigraphs<'a>(tokens: Vec<CharToken>) -> Vec<CharToken> {
    static REPLACEMENTS: &[(char, char)] = &[
        ('=', '#'),
        (')', ']'),
        ('!', '|'),
        ('(', '['),
        ('\'', '^'),
        ('>', '}'),
        ('/', '\\'),
        ('<', '{'),
        ('-', '~'),
    ];

    let mut output = Vec::new();
    let mut iter = tokens.into_iter();

    while iter.as_slice().len() > 2 {
        // advance iter by one
        let first = iter.next().unwrap();

        // peek ahead two extra tokens (after next)
        let second = &iter.as_slice()[0];
        let third = &iter.as_slice()[1];

        if first.value == '?' && second.value == '?' {
            if let Some((_, to)) = REPLACEMENTS.iter().find(|(from, _)| *from == third.value) {
                let mut loc = first.loc.clone();
                loc.len = 3;
                output.push(CharToken { value: *to, loc });
                iter.next();
                iter.next();
                continue;
            }
        }

        // did not find any trigraphs
        output.push(first);
    }

    while let Some(token) = iter.next() {
        output.push(token);
    }

    if log_enabled!(log::Level::Trace) {
        for (i, token) in output.iter().enumerate() {
            trace!("convert_trigraphs() output[{}] = {:?}", i, token);
        }
    }

    output
}

/// Phase 2: Splice together physical lines into logical lines
///
/// A line ending in `\` will be spliced together with the next line. Thus both
/// the back slash and newline characters will be removed. This allows multiline
/// comments and strings
pub fn splice_lines(tuctx: &mut TUCtx, input: Vec<CharToken>) -> Vec<CharToken> {
    let mut output = Vec::new();
    let mut iter = input.into_iter();

    while iter.as_slice().len() > 1 {
        let first = iter.next().unwrap();
        let second = &iter.as_slice()[0];

        if first.value == '\\' && second.value == '\n' {
            iter.next(); // consume second

            // do not emit either to output, in effect splicing physical lines
            // into one logical line

            // are these the last two characters of input?
            if iter.as_slice().len() == 0 {
                tuctx.emit_message(
                    first.loc.clone(),
                    MessageKind::Phase1FileEndingWithBackslash,
                );
            }
        } else {
            output.push(first);
        }
    }

    if let Some(last) = iter.next() {
        if last.value == '\\' {
            tuctx.emit_message(last.loc, MessageKind::Phase1FileEndingWithBackslash);
        } else {
            output.push(last);
        }
    }
    assert!(iter.next().is_none());

    output
}

/// Represents what type prefix was applied to a given string/character constant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Encoding {
    Default,
    Char16,
    Char32,
    WChar,
    UTF8,
}

impl Encoding {
    pub fn size_bytes(&self) -> usize {
        match *self {
            Encoding::Default => 1,
            Encoding::Char16 => 2,
            Encoding::Char32 => 4,
            Encoding::WChar => 4,
            Encoding::UTF8 => 1,
        }
    }

    pub fn type_str(&self) -> &'static str {
        match *self {
            Encoding::Default => "unsigned char",
            Encoding::Char16 => "char16_t",
            Encoding::Char32 => "char32_t",
            Encoding::WChar => "wchar_t",
            Encoding::UTF8 => "unsigned char",
        }
    }

    pub fn is_wide(&self) -> bool {
        match *self {
            Encoding::Char16 | Encoding::Char32 | Encoding::WChar => true,
            Encoding::Default | Encoding::UTF8 => false,
        }
    }

    pub fn compatible(&mut self, other: Encoding) -> bool {
        match (*self, other) {
            (Encoding::Default, new) => {
                *self = new;
                true
            },
            (_, Encoding::Default) => true,
            (old, new) => old == new,
        }
    }

    pub fn from_str(s: &str) -> Encoding {
        match s {
            "" => Encoding::Default,
            "u" => Encoding::Char16,
            "U" => Encoding::Char32,
            "L" => Encoding::WChar,
            "u8" => Encoding::UTF8,
            _ => unreachable!(), // should be handled by lexer
        }
    }

    pub fn to_str(&self) -> &'static str {
        match *self {
            Encoding::Default => "default",
            Encoding::Char16 => "universal 16",
            Encoding::Char32 => "universal 32",
            Encoding::WChar => "wide",
            Encoding::UTF8 => "utf-8",
        }
    }

    pub fn prefix(&self) -> &'static str {
        match *self {
            Encoding::Default => "",
            Encoding::Char16 => "u",
            Encoding::Char32 => "U",
            Encoding::WChar => "L",
            Encoding::UTF8 => "u8",
        }
    }
}

/// Represents what character came after the backslash in a numeric escape sequence
#[derive(Clone, Copy, Debug, PartialEq)]
enum DigitEscapePrefix {
    Universal16,
    Universal32,
    Hexadecimal,
    Octal,
}

impl DigitEscapePrefix {
    fn skip(&self) -> bool {
        use DigitEscapePrefix::*;
        match *self {
            Hexadecimal | Universal16 | Universal32 => true,
            Octal => false,
        }
    }

    fn radix(&self) -> u32 {
        use DigitEscapePrefix::*;
        match *self {
            Hexadecimal | Universal16 | Universal32 => 16,
            Octal => 8,
        }
    }

    fn max_len(&self) -> Option<usize> {
        use DigitEscapePrefix::*;
        match *self {
            Hexadecimal => None,
            Octal => Some(3),
            Universal16 => Some(4),
            Universal32 => Some(8),
        }
    }

    fn exact_len(&self) -> Option<usize> {
        use DigitEscapePrefix::*;
        match *self {
            Hexadecimal | Octal => None,
            Universal16 => Some(4),
            Universal32 => Some(8),
        }
    }

    fn as_str(&self) -> &'static str {
        use DigitEscapePrefix::*;
        match *self {
            Hexadecimal => "x",
            Octal => "",
            Universal16 => "u",
            Universal32 => "U",
        }
    }
}

/// Parse a numeric escape digit like `\x1234` or `\040`.
fn parse_digits(
    tuctx: &mut TUCtx,
    iter: &mut std::iter::Peekable<std::str::Chars>,
    buffer: &mut String,
    location: &Location,
    encoding: Encoding,
    prefix: DigitEscapePrefix,
) -> Option<char> {
    if prefix.skip() {
        // throw away the x, u, or U prefix for \x, \u, and \U escapes respectively
        iter.next();
    }

    while iter.peek().map(|c| c.is_digit(prefix.radix())) == Some(true) {
        buffer.push(iter.next().unwrap());

        // don't parse more than necessary for universal-character-name or octal-escape-sequence
        if let Some(max) = prefix.max_len() {
            if buffer.len() >= max {
                break;
            }
        }
    }

    if buffer.is_empty() {
        // TODO FIXME error reporting within escape sequences
        tuctx.emit_message(location.clone(), MessageKind::Phase5Empty);
        return None;
    }

    if prefix
        .exact_len()
        .map(|desired| buffer.len() < desired)
        .unwrap_or(false)
    {
        // detect an incomplete universal-character-name
        tuctx.emit_message(
            location.clone(), // TODO FIXME error reporting within escape sequences
            MessageKind::Phase5Incomplete {
                expected: prefix.exact_len().unwrap(),
                found: buffer.len(),
                prefix: if prefix == DigitEscapePrefix::Universal16 {
                    'u'
                } else {
                    'U'
                },
            },
        );
        return None;
    } else if prefix == DigitEscapePrefix::Hexadecimal && buffer.len() > encoding.size_bytes() * 2 {
        // detect a hexadecimal-escape-sequence that doesn't fit.
        // size is measured in bytes; there are 2 hexadecimal digits in a byte
        tuctx.emit_message(
            location.clone(), // TODO FIXME error reporting within escape sequences
            MessageKind::Phase5OutOfRange {
                prefix: prefix.as_str(),
                value: std::mem::take(buffer),
                encoding,
            },
        );
        return None;
    }

    let value = char::try_from(buffer.chars().fold(0u32, |current, next| {
        current * prefix.radix() + next.to_digit(prefix.radix()).unwrap()
    }));

    if let Ok(value) = value {
        Some(value)
    } else {
        tuctx.emit_message(
            location.clone(), // TODO FIXME error reporting within escape sequences
            MessageKind::Phase5Invalid {
                prefix: prefix.as_str(),
                value: std::mem::take(buffer),
            },
        );
        None
    }
}

/// Translate an input string/character constant
///
/// This will return `None` if the token has no escape codes. It will not
/// allocate in that case.
fn translate_escapes(
    tuctx: &mut TUCtx,
    text: &str,
    location: &Location,
    encoding: Encoding,
) -> Option<String> {
    // avoid allocating until we encounter first escape code
    // then we must rematerialize the `output` variable for all the previously
    // encountered characters
    let mut escaped = false;
    let mut output = String::new();

    let mut iter = text.chars().peekable();
    let mut i = 0;
    let mut buffer = String::new();

    while let Some(c) = iter.next() {
        i += 1;
        if c != '\\' {
            // not the beginning of an escape, ignore.

            if escaped {
                // we've encountered an escape already, therefore add this to output
                output.push(c);
            }
            continue;
        }

        // rematerialize what we've processed before into the output string
        if !escaped && output.is_empty() {
            // `!escaped` clause is to make sure this is only run for the very
            // first escape encountered (which may not result in a character if
            // it was erroneous )
            output = text.chars().take(i - 1).collect();
        }
        escaped = true;

        // just saw a backslash, beginning an escape
        let prefix = match iter.peek() {
            Some('x') => Some(DigitEscapePrefix::Hexadecimal), // hexadecimal-escape-sequence
            Some('u') => Some(DigitEscapePrefix::Universal16), // universal-character-name
            Some('U') => Some(DigitEscapePrefix::Universal32), // universal-character-name
            Some(c) if c.is_digit(8) => Some(DigitEscapePrefix::Octal), // octal-escape-sequence
            _ => None,
        };

        if let Some(prefix) = prefix {
            // the escape is made of some sequence of digits
            buffer.clear();
            if let Some(c) = parse_digits(tuctx, &mut iter, &mut buffer, location, encoding, prefix)
            {
                output.push(c);
            }
        } else {
            match iter.next() {
                // simple-escape-sequence
                Some('\\') => output.push('\\'),
                Some('?') => output.push('?'),
                Some('\'') => output.push('\''),
                Some('"') => output.push('"'),
                Some('a') => output.push('\x07'),
                Some('b') => output.push('\x08'),
                Some('f') => output.push('\x0c'),
                Some('n') => output.push('\x0a'),
                Some('r') => output.push('\x0d'),
                Some('t') => output.push('\x09'),
                Some('v') => output.push('\x0b'),

                Some(c) => {
                    // TODO FIXME error reporting within escape sequences
                    tuctx.emit_message(
                        location.clone(),
                        MessageKind::Phase5Unrecognized { escape: c },
                    );
                },
                None => {
                    // TODO FIXME error reporting within escape sequences
                    tuctx.emit_message(location.clone(), MessageKind::Phase5Empty);
                },
            }
        }
    }

    if output.is_empty() {
        None
    } else {
        Some(output)
    }
}

/// Mutate a [`PPToken`] to process escape sequences
///
/// By choosing `delim`, this function works for
/// [`CharacterConstants`](PPTokenKind::CharacterConstant) or
/// [`StringLiterals`](PPTokenKind::StringLiteral)
fn unescape_token(tuctx: &mut TUCtx, token: &mut PPToken, delim: &str) {
    debug_assert!(delim.as_bytes().len() == 1);

    // token.value includes any prefix and delimiters
    let mut split = token.value.split(delim);
    let prefix = split.next().unwrap();

    let start = prefix.as_bytes().len() + 1; // remove leading delim
    let end = token.value.as_bytes().len() - 1; // remove trailing delim
    let text = &token.value[start..end];

    if let Some(value) = translate_escapes(tuctx, text, &token.location, Encoding::from_str(prefix))
    {
        token.value = format!("{}{}{}{}", prefix, delim, value, delim);
    }
}

/// Phase 5: Process escape sequences
pub fn unescape(tuctx: &mut TUCtx, input: &mut Vec<PPToken>) {
    for token in input {
        match token.kind {
            PPTokenKind::StringLiteral => unescape_token(tuctx, token, "\""),
            PPTokenKind::CharacterConstant => unescape_token(tuctx, token, "\'"),
            _ => continue,
        }
    }
}

/// Get encoding prefix for string/character constant
pub fn get_string_encoding(s: &str, delim: &str) -> Encoding {
    let prefix = s.split(delim).next().unwrap();
    Encoding::from_str(prefix)
}

/// Get content of string/character constant
pub fn get_string_content<'a>(s: &'a str, delim: &str) -> &'a str {
    let prefix_len = s.split(delim).next().unwrap().as_bytes().len();
    let len = s.as_bytes().len();
    &s[prefix_len + 1..len - 1]
}

/// Phase 6: Concatenate adjacent string literals and remove whitespace
pub fn concatenate(tuctx: &mut TUCtx, input: Vec<PPToken>) -> Vec<PPToken> {
    let mut iter = input.into_iter().filter(|t| !t.is_whitespace()).peekable();
    let mut output = Vec::new();

    while let Some(mut token) = iter.next() {
        if token.kind != PPTokenKind::StringLiteral {
            output.push(token);
        } else {
            // Encoding::compatible() will update the overall encoding of this string if it was
            // previously default-encoded.
            let mut encoding = get_string_encoding(&token.value, "\"");
            let mut string = get_string_content(&token.value, "\"").to_owned();

            while iter.peek().map(|t| t.kind == PPTokenKind::StringLiteral) == Some(true) {
                let new_token = iter.next().unwrap();
                let new_encoding = get_string_encoding(&new_token.value, "\"");

                if encoding.compatible(new_encoding) {
                    // TODO FIXME expand token locations???
                    string.push_str(get_string_content(&new_token.value, "\""));
                } else {
                    tuctx.emit_message(
                        new_token.location,
                        MessageKind::Phase6IncompatibleEncoding {
                            previous: encoding,
                            current: new_encoding,
                        },
                    )
                }
            }

            token.value = format!("{}\"{}\"", encoding.prefix(), string);

            output.push(token);
        }
    }

    output
}
