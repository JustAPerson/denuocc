// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

use crate::target::Target;

/// The possible types of a character constant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CharacterType {
    UnsignedChar,
    WideChar, // L
    Char16,   // u
    Char32,   // U
}

impl std::fmt::Display for CharacterType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            CharacterType::UnsignedChar => write!(f, "unsigned char"),
            CharacterType::WideChar => write!(f, "wchar_t"),
            CharacterType::Char16 => write!(f, "char16_t"),
            CharacterType::Char32 => write!(f, "char32_t"),
        }
    }
}

/// The possible types of a string literal
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StringType {
    UnsignedChar,
    WideChar, // L
    Char16,   // u
    Char32,   // U
    Utf8,     // u8
}

impl std::fmt::Display for StringType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            StringType::UnsignedChar => write!(f, "unsigned char"),
            StringType::WideChar => write!(f, "wchar_t"),
            StringType::Char16 => write!(f, "char16_t"),
            StringType::Char32 => write!(f, "char32_t"),
            StringType::Utf8 => write!(f, "unsigned char (utf-8 coded)"),
        }
    }
}

/// The possible types of a float constant
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FloatType {
    Float,
    Double,
    LongDouble, // TODO long double repr depends on platform? choose x86 for now
}

impl FloatType {
    /// Return the size of this type in bytes
    pub fn sizeof(&self) -> u32 {
        match *self {
            FloatType::Float => 4,
            FloatType::Double => 8,
            FloatType::LongDouble => 10,
        }
    }

    /// Return the number of bits of the significand that are stored in the
    /// binary representation
    pub fn significand_stored_bits(&self) -> u32 {
        match *self {
            FloatType::Float => 23,
            FloatType::Double => 52,
            FloatType::LongDouble => 64,
        }
    }

    /// Return the number of bits of accuracy the significand is implied to have
    /// for normal numbers
    pub fn significand_implied_bits(&self) -> u32 {
        self.significand_stored_bits() + 1
    }

    /// Return the number of bits of dedicated to the exponent
    pub fn exponent_bits(&self) -> u32 {
        match *self {
            FloatType::Float => 8,
            FloatType::Double => 11,
            FloatType::LongDouble => 15,
        }
    }

    /// Return the minimum value exponent that can be stored for normal numbers.
    ///
    /// Note: subnormal numbers may achieve a lower notional exponent.
    pub fn exponent_min(&self) -> i32 {
        match *self {
            FloatType::Float => -126,
            FloatType::Double => -1022,
            FloatType::LongDouble => -32766,
        }
    }

    /// Return the maximum value exponent
    pub fn exponent_max(&self) -> i32 {
        match *self {
            FloatType::Float => 127,
            FloatType::Double => 1023,
            FloatType::LongDouble => 32767,
        }
    }

    /// Return the bias that should be applied to the signed exponent before
    /// storing it in the binary representation
    pub fn exponent_bias(&self) -> i32 {
        self.exponent_max()
    }
}

impl std::fmt::Display for FloatType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            FloatType::Float => write!(f, "float"),
            FloatType::Double => write!(f, "double"),
            FloatType::LongDouble => write!(f, "long double"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IntegerType {
    UnsignedInt,
    UnsignedShort,
    UnsignedLong,
    UnsignedLongLong,
    SignedInt,
    SignedShort,
    SignedLong,
    SignedLongLong,
}

static INTEGER_CONSTANT_TYPES: &[IntegerType] = &[
    IntegerType::SignedInt,
    IntegerType::UnsignedInt,
    IntegerType::SignedLong,
    IntegerType::UnsignedLong,
    IntegerType::SignedLongLong,
    IntegerType::UnsignedLongLong,
];

impl IntegerType {
    /// Return the maximum value of this type
    pub fn max_value(&self, target: &dyn Target) -> u64 {
        target.integer_max_value(*self)
    }

    /// Return the minimum value of this type
    pub fn min_value(&self, target: &dyn Target) -> i64 {
        target.integer_min_value(*self)
    }

    /// Return the size of this type in bytes
    pub fn sizeof(&self, target: &dyn Target) -> usize {
        target.integer_sizeof(*self)
    }

    /// Return true if this type is signed
    pub fn signed(&self) -> bool {
        use IntegerType::*;
        match *self {
            UnsignedInt | UnsignedShort | UnsignedLong | UnsignedLongLong => false,
            SignedInt | SignedShort | SignedLong | SignedLongLong => true,
        }
    }

    /// Return true if this type is unsigned
    pub fn unsigned(&self) -> bool {
        !self.signed()
    }

    /// Given a value `v` from a decimal constant, return the deduced type
    /// for the given `target`
    ///
    /// `ls` should be 1 if the suffix included `"l"` or `"L"`.<br/>
    /// `ls` should be 2 if the suffix included `"ll"` or `"LL"`.<br/>
    /// `ls` should be 0 otherwise.<br/>
    /// `u` should be true if the suffix included `"u"`<br/>
    pub fn deduce_type_dec(v: u64, ls: usize, u: bool, target: &dyn Target) -> Option<Self> {
        debug_assert!(ls <= 2);
        INTEGER_CONSTANT_TYPES
            .iter()
            .filter(|t| t.unsigned() == u)
            .skip(ls)
            .filter(|t| t.max_value(target) >= v)
            .next()
            .copied()
    }

    /// Similar to [`IntegerType::deduce_type_dec`] except for octal or
    /// hexadecimal constants, which have different type deduction rules.
    pub fn deduce_type_nondec(v: u64, ls: usize, u: bool, target: &dyn Target) -> Option<Self> {
        debug_assert!(ls <= 2);
        INTEGER_CONSTANT_TYPES
            .iter()
            .filter(|t| if u { t.unsigned() } else { true })
            .skip(if u { ls } else { 2 * ls })
            .filter(|t| t.max_value(target) >= v)
            .next()
            .copied()
    }
}

impl std::fmt::Display for IntegerType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            IntegerType::UnsignedInt => write!(f, "unsigned int"),
            IntegerType::UnsignedShort => write!(f, "unsigned short int"),
            IntegerType::UnsignedLong => write!(f, "unsigned long int"),
            IntegerType::UnsignedLongLong => write!(f, "unsigned long long int"),
            IntegerType::SignedInt => write!(f, "int"),
            IntegerType::SignedShort => write!(f, "short int"),
            IntegerType::SignedLong => write!(f, "long int"),
            IntegerType::SignedLongLong => write!(f, "long long int"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_integertype_deduce_type_dec() {
        fn deduce(v: u64, ls: usize, u: bool) -> Option<IntegerType> {
            let target = crate::target::linux::Linux;
            IntegerType::deduce_type_dec(v, ls, u, &target)
        }

        use IntegerType::*;

        assert_eq!(deduce(0, 0, false), Some(SignedInt));
        assert_eq!(deduce(0, 1, false), Some(SignedLong));
        assert_eq!(deduce(0, 2, false), Some(SignedLongLong));
        assert_eq!(deduce(0, 0, true), Some(UnsignedInt));
        assert_eq!(deduce(0, 1, true), Some(UnsignedLong));
        assert_eq!(deduce(0, 2, true), Some(UnsignedLongLong));

        assert_eq!(deduce(0x8000_0000, 0, false), Some(SignedLong));
        assert_eq!(deduce(0x8000_0000, 0, true), Some(UnsignedInt));
        assert_eq!(deduce(0x7fff_ffff_ffff_ffff, 0, false), Some(SignedLong));
        assert_eq!(
            deduce(0x7fff_ffff_ffff_ffff, 2, false),
            Some(SignedLongLong)
        );
        assert_eq!(deduce(0x8000_0000_0000_0000, 0, false), None);
        assert_eq!(deduce(0x8000_0000_0000_0000, 0, true), Some(UnsignedLong));
        assert_eq!(
            deduce(0x8000_0000_0000_0000, 2, true),
            Some(UnsignedLongLong)
        );
    }

    #[test]
    fn test_integertype_deduce_type_nondec() {
        fn deduce(v: u64, ls: usize, u: bool) -> Option<IntegerType> {
            let target = crate::target::linux::Linux;
            IntegerType::deduce_type_nondec(v, ls, u, &target)
        }

        use IntegerType::*;

        assert_eq!(deduce(0, 0, false), Some(SignedInt));
        assert_eq!(deduce(0, 1, false), Some(SignedLong));
        assert_eq!(deduce(0, 2, false), Some(SignedLongLong));
        assert_eq!(deduce(0, 0, true), Some(UnsignedInt));
        assert_eq!(deduce(0, 1, true), Some(UnsignedLong));
        assert_eq!(deduce(0, 2, true), Some(UnsignedLongLong));

        assert_eq!(deduce(0x7fff_ffff, 0, false), Some(SignedInt));
        assert_eq!(deduce(0x7fff_ffff, 0, true), Some(UnsignedInt));
        assert_eq!(deduce(0x8000_0000, 0, false), Some(UnsignedInt));
        assert_eq!(deduce(0x8000_0000, 1, false), Some(SignedLong));
    }
}
