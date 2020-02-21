// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

use log::trace;

use crate::front::message::MessageKind;
use crate::front::token::PPToken;
use crate::front::types::{CharacterType, FloatType, IntegerType, StringType};
use crate::tu::TUCtx;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Radix {
    Decimal,
    Hexadecimal,
    Octal,
}

impl Radix {
    pub fn value(&self) -> u32 {
        match *self {
            Radix::Decimal => 10,
            Radix::Hexadecimal => 16,
            Radix::Octal => 8,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match *self {
            Radix::Decimal => "decimal",
            Radix::Hexadecimal => "hexadecimal",
            Radix::Octal => "octal",
        }
    }
}

impl std::fmt::Display for Radix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Character {
    pub value: char,
    pub datatype: CharacterType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct String {
    pub value: std::string::String,
    pub datatype: StringType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FloatData {
    Float([u8; 4]),
    Double([u8; 8]),
    LongDouble([u8; 10]),
}

impl FloatData {
    pub fn from_f32(f: f32) -> Self {
        FloatData::Float(f.to_le_bytes())
    }

    pub fn from_f64(f: f64) -> Self {
        FloatData::Double(f.to_le_bytes())
    }

    pub fn from_bits(bits: &u128, datatype: FloatType) -> Self {
        let bytes = bits.to_le_bytes(); // TODO endianness floats
        match datatype {
            FloatType::Float => {
                let mut buffer = [0; 4];
                buffer.copy_from_slice(&bytes[..4]);
                FloatData::Float(buffer)
            }
            FloatType::Double => {
                let mut buffer = [0; 8];
                buffer.copy_from_slice(&bytes[..8]);
                FloatData::Double(buffer)
            }
            FloatType::LongDouble => {
                let mut buffer = [0; 10];
                buffer.copy_from_slice(&bytes[..10]);
                FloatData::LongDouble(buffer)
            }
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match *self {
            FloatData::Float(bytes) => Some(f32::from_le_bytes(bytes)),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            FloatData::Double(bytes) => Some(f64::from_le_bytes(bytes)),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Float {
    pub value: FloatData,
    pub datatype: FloatType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IntegerData {
    Signed(i64),
    Unsigned(u64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Integer {
    pub value: IntegerData,
    pub datatype: IntegerType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumberKind {
    Float,
    Integer,
}

impl std::fmt::Display for NumberKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            NumberKind::Float => write!(f, "float"),
            NumberKind::Integer => write!(f, "integer"),
        }
    }
}

pub enum Number {
    Float(Float),
    Integer(Integer),
}

/// Determine if a string is a float constant
fn is_float(value: &str) -> bool {
    let mut iter = value.chars().peekable();

    /// Consume all digits of iterator that are in this radix
    fn consume_radix(iter: &mut std::iter::Peekable<impl Iterator<Item = char>>, radix: u32) {
        while iter.peek().map(|c| c.is_digit(radix)) == Some(true) {
            iter.next();
        }
    }

    if value.starts_with("0x") || value.starts_with("0X") {
        iter.next(); // consume 0
        iter.next(); // consume x/X
        consume_radix(&mut iter, 16); // consume whole part of significand

        if iter.peek() == Some(&'.') {
            iter.next(); // consume decimal point
            consume_radix(&mut iter, 16); // consume fractional part of significand
            return true;
        }
        if iter.peek().map(|c| c.to_ascii_lowercase() == 'p') == Some(true) {
            return true;
        }
    } else {
        consume_radix(&mut iter, 10); // consume whole part of significand
        if iter.peek() == Some(&'.') {
            iter.next(); // consume decimal point
            consume_radix(&mut iter, 10); // consume fractional part of significand
            return true;
        }
        if iter.peek().map(|c| c.to_ascii_lowercase() == 'e') == Some(true) {
            return true;
        }
    }

    false
}

/// Returns the type of a float constant as specified by its suffix
///
/// Returns an error if a suffix is unrecognized, repeated, or conflicts (such
/// as specifying both `f` for `float` and `l` for `long double`).
fn parse_float_suffix(mut suffix: &str, radix: Radix) -> Result<FloatType, MessageKind> {
    let mut float = false;
    let mut long = false;

    while !suffix.is_empty() {
        if suffix.starts_with("f") || suffix.starts_with("F") {
            if float {
                return Err(MessageKind::Phase7NumberSuffixRepeat);
            }
            float = true;
        } else if suffix.starts_with("l") || suffix.starts_with("L") {
            if long {
                return Err(MessageKind::Phase7NumberSuffixRepeat);
            }
            long = true;
        } else {
            return Err(MessageKind::Phase7NumberSuffixUnknown {
                suffix: suffix.to_owned(),
                radix,
                numkind: NumberKind::Float,
            });
        }
        suffix = &suffix[1..];
    }

    match (float, long) {
        (true, true) => Err(MessageKind::Phase7NumberSuffixConflict {
            radix,
            numkind: NumberKind::Float,
        }),
        (true, _) => Ok(FloatType::Float),
        (_, true) => Ok(FloatType::LongDouble),
        (false, false) => Ok(FloatType::Double),
    }
}

/// Returns the smallest number of bits required to represent this number
fn occupied_bits(v: u64) -> u32 {
    64 - v.leading_zeros()
}

/// Use and modify `remaining` to find the next digit sequence
///
/// Returns a slice of the `remaining` string that is digits in the
/// corresponding `radix` base. If `sign` is true, an optional +/- may be
/// accommodated. `remaining` is then updated to point after the returned
/// slice.
fn take_digits<'a>(remaining: &mut &'a str, radix: u32, sign: bool) -> &'a str {
    // handle leading +/- sign
    let mut bytes = 0;
    if sign && (remaining.chars().next().map(|c| ['+', '-'].contains(&c)) == Some(true)) {
        bytes = 1;
    }
    bytes += remaining
        .chars()
        .skip(bytes)
        .take_while(|c| c.is_digit(radix))
        .count();
    let s = &(*remaining)[..bytes];
    *remaining = &(*remaining)[bytes..];
    s
}

/// Like [`take_digits`](take_digits) except it will only consume a digit
/// sequence if it is preceded by something in the `cond_set`
fn take_digits_if<'a>(
    remaining: &mut &'a str,
    cond_set: &[char],
    radix: u32,
    sign: bool,
) -> &'a str {
    remaining
        .chars()
        .next()
        .filter(|c| cond_set.contains(c))
        .map(|_| {
            (*remaining) = &(*remaining)[1..]; // skip whatever we conditioned on
            take_digits(remaining, radix, sign)
        })
        .unwrap_or("")
}

/// Converts a series of hex digits into a number
///
/// If the number is too large to represent in a `u64`, we return only the most
/// significant bits of the number. If this occurs, the second value returned
/// will be true.
fn parse_hex_prefix(input: &str) -> (u64, bool) {
    debug_assert!(input.chars().all(|c| c.is_digit(16)));
    if input.is_empty() {
        (0, false)
    } else if let Ok(v) = u64::from_str_radix(input, 16) {
        (v, false)
    } else {
        // now attempt to parse some prefix of the input as a u64, throwing away
        // any detail and report that we truncated. Start by trying prefixes
        // working backwards from end of string. We already know the full string
        // does not parse.
        for i in (0..input.len()).rev() {
            if let Ok(v) = u64::from_str_radix(&input[..i], 16) {
                return (v, true);
            }
        }
        unreachable!();
    }
}

/// Converts a series of hex digits into a number, only returning the specified
/// bits worth
///
/// The first value returned is the result it self. The second value is the
/// number of significant bits returned. The last boolean is true if there were
/// more significand bits than desired.
fn parse_hex_leading_bits(input: &str, desired_bits: u32) -> (u64, u32, bool) {
    let (mut input, mut truncate) = parse_hex_prefix(input);
    let mut input_bits = occupied_bits(input);
    if input_bits > desired_bits {
        input >>= input_bits - desired_bits;
        truncate |= true;
        input_bits = desired_bits;
    }
    (input, input_bits, truncate)
}

/// Parse hexadecimal float constants from string
fn parse_hex_float_constant(input: &str) -> Result<Float, MessageKind> {
    let mut remaining = &input[2..];
    let whole: &str = take_digits(&mut remaining, 16, false);
    let fraction: &str = take_digits_if(&mut remaining, &['.'], 16, false);
    let exponent: &str = take_digits_if(&mut remaining, &['p', 'P'], 10, true);

    trace!(
        "parse_hex_float_constant() whole={:?} fraction={:?} exponent={:?} suffix={:?}",
        whole,
        fraction,
        exponent,
        remaining
    );

    if whole.is_empty() && fraction.is_empty() {
        return Err(MessageKind::Phase7FloatNoSignificand {
            radix: Radix::Hexadecimal,
        });
    } else if exponent.is_empty() {
        return Err(MessageKind::Phase7FloatNoExponent);
    }

    let suffix_type = parse_float_suffix(remaining, Radix::Hexadecimal)?;
    let implied_bits = suffix_type.significand_implied_bits();
    let (w, wbits, wtruncate) = parse_hex_leading_bits(whole, implied_bits);
    let (f, fbits, ftruncate) = parse_hex_leading_bits(fraction, implied_bits - wbits);

    trace!("parse_hex_float_constant() suffix_type={:?}", suffix_type);
    trace!(
        "parse_hex_float_constant() w={:#x} wbits={} wtruncate={}",
        w,
        wbits,
        wtruncate
    );
    trace!(
        "parse_hex_float_constant() f={:#x} fbits={} ftruncate={}",
        f,
        fbits,
        ftruncate
    );

    if wtruncate | ftruncate {
        return Err(MessageKind::Phase7HexFloatUnrepresentable {
            value: input.to_owned(),
            datatype: suffix_type,
        });
    }

    // from_str_radix returns Err if input is empty string
    let mut e = i32::from_str_radix(exponent, 10).unwrap_or(1_000_000);

    let mut raw_significand;
    let raw_exponent;

    if wbits == 0 && fbits == 0 {
        // significand is zero, which has a special encoding
        raw_significand = 0;
        raw_exponent = 0;
    } else {
        let possible_fbits = fraction.len() as u32 * 4; // hex digits are 4 bits each
        let total_bits;

        if wbits > 0 {
            // there's something to left of decimal point
            e += wbits as i32 - 1;
            raw_significand = w << possible_fbits | f;
            total_bits = wbits + possible_fbits;
        } else {
            debug_assert!(fbits > 0);
            // imagine the float `0x0.04p0`. The binary encoding of the
            // fraction is `00000100`. The `used_fbits` variable will be 3,
            // but since we want a normal float to have an implied 1 bit to
            // left of the decimal point, we adjust the exponent by (8 - 3 +
            // 1) = 6.
            let leading_fzeros = possible_fbits - fbits;
            e -= leading_fzeros as i32 + 1;

            // now we shift f to occupy the significand
            raw_significand = f;
            total_bits = fbits;
        }

        // make leading 1 of significand line up with its implied position.
        // we will mask this off for normal numbers, or we might shift it right
        // for subnormals
        raw_significand <<= suffix_type.significand_implied_bits() - total_bits;

        trace!("parse_hex_float_constant() e={}", e);
        if e < suffix_type.exponent_min() {
            // subnormal
            let subnormal_offset = suffix_type.exponent_min() - e;
            trace!(
                "parse_hex_float_constant() subnormal_offset={}",
                subnormal_offset
            );

            if subnormal_offset > raw_significand.trailing_zeros() as i32 {
                // would chop off a significant bit when right shifting.

                // standard/gcc/clang imply we are wrong here and that we should
                // divide/round significand itself rather than shifting it.
                // See comment in self::test::test_parse_hex_float_constant().
                return Err(MessageKind::Phase7HexFloatUnrepresentable {
                    value: input.to_owned(),
                    datatype: suffix_type,
                });
            }

            raw_exponent = 0;
            raw_significand >>= subnormal_offset;
        } else if e > suffix_type.exponent_max() {
            return Err(MessageKind::Phase7HexFloatUnrepresentable {
                value: input.to_owned(),
                datatype: suffix_type,
            });
        } else {
            // mask off implied leading 1
            raw_significand &= (1 << suffix_type.significand_stored_bits()) - 1;
            raw_exponent = (e + suffix_type.exponent_bias()) as u64;
        }
    };

    trace!(
        "parse_hex_float_constant() raw_significand={:#x} raw_exponent={:#x}",
        raw_significand,
        raw_exponent
    );
    debug_assert!(occupied_bits(raw_significand) <= suffix_type.significand_implied_bits());
    debug_assert!(occupied_bits(raw_exponent) <= suffix_type.exponent_bits());

    let mut bigbits: u128 = 0;
    bigbits |= (raw_exponent << suffix_type.significand_stored_bits()) as u128;
    bigbits |= raw_significand as u128;
    trace!("parse_hex_float_constant() bigbits={:#x}", bigbits);

    Ok(Float {
        value: FloatData::from_bits(&bigbits, suffix_type),
        datatype: suffix_type,
    })
}

/// Parse decimal float constants from string
fn parse_dec_float_constant(input: &str) -> Result<Float, MessageKind> {
    use std::str::FromStr;

    let mut remaining = input;
    let whole: &str = take_digits(&mut remaining, 10, false);
    let fraction: &str = take_digits_if(&mut remaining, &['.'], 10, false);
    let _exponent: &str = take_digits_if(&mut remaining, &['e', 'E'], 10, true);

    if whole.is_empty() && fraction.is_empty() {
        return Err(MessageKind::Phase7FloatNoSignificand {
            radix: Radix::Hexadecimal,
        });
    }

    // find the actual digits of the float that we can parse using the Rust stdlib
    // TODO write our own strtod() algorithm
    // See "How to Read Floating Point Numbers Accurately" by William Clinger
    let float_len = input.as_bytes().len() - remaining.as_bytes().len();
    let float = &input[..float_len];

    let suffix_type = parse_float_suffix(remaining, Radix::Decimal)?;
    let value = match suffix_type {
        FloatType::Float => f32::from_str(float).map(|f| FloatData::from_f32(f)),
        FloatType::Double => f64::from_str(float).map(|f| FloatData::from_f64(f)),
        FloatType::LongDouble => unimplemented!("parsing decimal long doubles"),
    };

    if value.is_err() {
        // rust stdlib parsing can fail for extremes
        // see https://github.com/rust-lang/rust/issues/31407
        return Err(MessageKind::Phase7DecFloatFailure {
            value: input.to_owned(),
            datatype: suffix_type,
        });
    }

    Ok(Float {
        value: value.unwrap(),
        datatype: suffix_type,
    })
}

/// Parse float constants from a [`PPToken`](PPToken)
fn parse_float_constant(tuctx: &mut TUCtx, token: &PPToken) -> Result<Number, MessageKind> {
    let value = token.value.as_str();

    let float;
    if value.starts_with("0x") || value.starts_with("0X") {
        float = parse_hex_float_constant(value)?;
    } else {
        float = parse_dec_float_constant(value)?;
    }

    Ok(Number::Float(float))
}

/// Returns the type of a integer constant as specified by its suffix
///
/// Returns an error if a suffix is unrecognized, repeated, or conflicts (such
/// as specifying both `l` for `long` and `ll` for `long long`).
///
/// The returned tuple represents if the result is unsigned and how many `long`
/// type qualifiers should be applied.
fn parse_integer_suffix(mut suffix: &str, radix: Radix) -> Result<(bool, usize), MessageKind> {
    let mut unsigned = false;
    let mut long = false;
    let mut longlong = false;

    while !suffix.is_empty() {
        if suffix.starts_with("u") || suffix.starts_with("U") {
            if unsigned {
                return Err(MessageKind::Phase7NumberSuffixRepeat);
            }
            unsigned = true;
            suffix = &suffix[1..];
        } else if suffix.starts_with("ll") || suffix.starts_with("LL") {
            if longlong {
                return Err(MessageKind::Phase7NumberSuffixRepeat);
            }
            longlong = true;
            suffix = &suffix[2..];
        } else if suffix.starts_with("l") || suffix.starts_with("L") {
            if long {
                return Err(MessageKind::Phase7NumberSuffixRepeat);
            }
            long = true;
            suffix = &suffix[1..];
        } else {
            return Err(MessageKind::Phase7NumberSuffixUnknown {
                suffix: suffix.to_owned(),
                radix,
                numkind: NumberKind::Float,
            });
        }
    }

    if long && longlong {
        return Err(MessageKind::Phase7NumberSuffixConflict {
            radix,
            numkind: NumberKind::Integer,
        });
    }

    Ok((
        unsigned,
        if longlong {
            2
        } else if long {
            1
        } else {
            0
        },
    ))
}

fn parse_integer_constant(tuctx: &mut TUCtx, token: &PPToken) -> Result<Number, MessageKind> {
    use Radix::*;
    let mut remaining = token.value.as_str();

    let (skip, radix) = match remaining {
        v if v.starts_with("0x") || v.starts_with("0X") => (2, Hexadecimal),
        v if v.starts_with("0") => (1, Octal),
        _ => (0, Decimal),
    };
    remaining = &remaining[skip..];

    let digits = take_digits(&mut remaining, radix.value(), false);
    let (u, ls) = parse_integer_suffix(remaining, radix)?;
    let value =
        u64::from_str_radix(digits, radix.value()).map_err(|_| MessageKind::Phase7IntTooLarge {
            value: token.value.clone(),
        })?;

    let suffix_type = match radix {
        Decimal => IntegerType::deduce_type_dec(value, ls, u, tuctx.driver().target()),
        Hexadecimal | Octal => {
            IntegerType::deduce_type_nondec(value, ls, u, tuctx.driver().target())
        }
    }
    .ok_or_else(|| MessageKind::Phase7IntTooLarge {
        value: token.value.clone(),
    })?;

    Ok(Number::Integer(Integer {
        value: match suffix_type.signed() {
            true => IntegerData::Signed(value as i64),
            false => IntegerData::Unsigned(value),
        },
        datatype: suffix_type,
    }))
}

pub fn parse_number_constant(tuctx: &mut TUCtx, token: &PPToken) -> Option<Number> {
    let number;
    if is_float(&token.value) {
        number = parse_float_constant(tuctx, token);
    } else {
        number = parse_integer_constant(tuctx, token)
    };

    match number {
        Ok(number) => Some(number),
        Err(e) => {
            tuctx.emit_message(token.location.clone(), e);
            None
        }
    }
}

pub fn parse_string_constant(tuctx: &mut TUCtx, token: &PPToken) -> Option<String> {
    None
}

pub fn parse_character_constant(tuctx: &mut TUCtx, token: &PPToken) -> Option<Character> {
    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_is_float() {
        assert!(!is_float("0"));
        assert!(!is_float("0xa"));
        assert!(!is_float("078"));
        assert!(!is_float("1l"));
        assert!(!is_float("1u"));
        assert!(!is_float("1ull"));

        assert!(is_float("1."));
        assert!(is_float("1.0"));
        assert!(is_float("10e3"));
        assert!(is_float("10e3f"));
        assert!(is_float("10e3l"));
        assert!(is_float("0x10p3"));
    }

    #[test]
    fn test_parse_hex_float_constant() {
        fn pf32(input: &str) -> Result<f32, MessageKind> {
            parse_hex_float_constant(input).map(|f| f.value.as_f32().unwrap())
        }
        fn pf64(input: &str) -> Result<f64, MessageKind> {
            parse_hex_float_constant(input).map(|f| f.value.as_f64().unwrap())
        }

        assert_eq!(pf64("0x0p0").unwrap().to_bits(), 0x0);

        assert_eq!(pf32("0x0p0f").unwrap().to_bits(), 0x0);

        assert_eq!(pf64("0x.1p4").unwrap(), 1.0);
        assert_eq!(pf64("0x8p4").unwrap(), 128.0);
        assert_eq!(pf64("0x7p-2").unwrap(), 1.75);
        assert_eq!(pf64("0x1p-1022").unwrap().to_bits(), 0x0010000000000000); // smallest normal
        assert_eq!(pf64("0x1p-1074").unwrap().to_bits(), 0x0000000000000001); // smallest subnormal
        assert_eq!(pf64("0x1p1023").unwrap().to_bits(), 0x7fe0000000000000); // largest exponent
        assert_eq!(
            pf64("0x1.fffffffffffffp1023").unwrap().to_bits(),
            0x7fefffffffffffff
        ); // largest value

        // unrepresentable
        assert!(pf64("0x1p-1075").is_err()); // smaller than smallest subnormal
        assert!(pf64("0x1p1024").is_err()); // too large
        assert!(pf64("0x2p1023").is_err()); // too large
        assert!(pf64("0x0.8p1024").is_ok());

        // The following is iffy. This cannot be represented exactly.
        // Just shifting to handle subnormals would chop off a significant bit
        // The standard/gcc/clang imply we should divide & round significand from 3 to 2
        // Thus ending up with the same value as `0x2p-1074`.
        assert!(pf64("0x3p-1075").is_err());
    }

    #[test]
    fn test_parse_dec_float_constant() {
        // TODO include more float tests from here
        // https://github.com/ahrvoje/numerics/blob/master/strtod/strtod_tests.toml

        fn pf32(input: &str) -> Result<f32, MessageKind> {
            parse_dec_float_constant(input).map(|f| f.value.as_f32().unwrap())
        }
        fn pf64(input: &str) -> Result<f64, MessageKind> {
            parse_dec_float_constant(input).map(|f| f.value.as_f64().unwrap())
        }

        assert_eq!(pf64("123.456").unwrap(), 123.456);
        assert_eq!(pf32("123.456f").unwrap(), 123.456f32);

        assert_eq!(pf64("1.3e-300").unwrap(), 1.3e-300);
        assert_eq!(pf64("1.3e-300").unwrap(), 1.3e-300);

        assert_eq!(
            pf64("1.7976931348623157E+308").unwrap().to_bits(),
            0x7fefffffffffffff
        ); // largest value
        assert_eq!(
            pf64("8.9884656743115795E+307").unwrap().to_bits(),
            0x7fe0000000000000
        ); // largest power of 2
        assert_eq!(
            pf64("2.2250738585072014E-308").unwrap().to_bits(),
            0x0010000000000000
        ); // smallest power of 2
        assert_eq!(
            pf64("2.2250738585072009E-308").unwrap().to_bits(),
            0x000fffffffffffff
        ); // largest subnormal
        assert_eq!(
            pf64("1.1125369292536007E-308").unwrap().to_bits(),
            0x0008000000000000
        ); // midpoint subnormal
        assert_eq!(
            pf64("4.9406564584124654E-324").unwrap().to_bits(),
            0x0000000000000001
        ); // smallest subnormal

        assert!(pf64("2E+308").unwrap().is_infinite()); // too large
        assert_eq!(pf64("2E-324").unwrap(), 0.0); // too small
    }
}
