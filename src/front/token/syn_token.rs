// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

// TODO redo {KEYWORD,PUNCTUATOR}_MAPS with string interning
// TODO benchmark {KEYWORD,PUNCTUATOR}_MAPS with phf crate

use std::collections::HashMap;

use bitflags::bitflags;
use lazy_static::lazy_static;

use crate::front::location::{Location, Span};

#[derive(Clone, Debug)]
pub struct SynToken {
    pub location: Location,
    pub kind: SynTokenKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SynTokenKind {
    Identifier(String),
    String(String),
    Character(String),
    Number(String),
    Keyword(FastKeyword),
    Punctuator(Punctuator),
}

impl SynToken {
    pub fn is_keyword(&self, keyword: Keyword) -> bool {
        match self.kind {
            SynTokenKind::Keyword(fk) => fk.keyword == keyword,
            _ => false,
        }
    }

    pub fn is_punctuator(&self, punctuator: Punctuator) -> bool {
        match self.kind {
            SynTokenKind::Punctuator(p) => p == punctuator,
            _ => false,
        }
    }

    pub fn span_kind(self) -> Span<SynTokenKind> {
        Span {
            location: self.location,
            value: self.kind,
        }
    }

    pub fn span_value<T>(self, value: T) -> Span<T> {
        Span {
            location: self.location,
            value,
        }
    }
}

impl std::ops::Deref for SynToken {
    type Target = SynTokenKind;
    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

macro_rules! is_methods {
    ($(($method:ident, $variant:ident)),+) => ($(
        pub fn $method(&self) -> bool {
            match *self {
                SynTokenKind::$variant(_) => true,
                _ => false,
            }
        }
    )+)
}

macro_rules! as_methods {
    ($(($method:ident, $variant:ident, $type:ty)),+) => ($(
        pub fn $method(&self) -> Option<$type> {
            match self {
                SynTokenKind::$variant(value) => Some(value),
                _ => None,
            }
        }
    )+)
}

impl SynTokenKind {
    is_methods! {
        (is_ident, Identifier),
        (is_string, String),
        (is_char, Character),
        (is_number, Number)
    }

    as_methods! {
        (as_ident, Identifier, &str),
        (as_string, String, &str),
        (as_char, Character, &str),
        (as_number, Number, &str),
        (as_keyword, Keyword, &FastKeyword),
        (as_punctuator, Punctuator, &Punctuator)
    }
}

bitflags! {
    pub struct KeywordCategory: u8 {
        const STORAGE_CLASS_SPECIFIER = 0b0000_0001;
        const TYPE_SPECIFIER          = 0b0000_0010;
        const SIMPLE_TYPE_SPECIFIER   = 0b0000_0110;
        const TYPE_QUALIFIER          = 0b0000_1000;

        const LABELED_STATEMENT       = 0b0001_0000;
        const SELECTION_STATEMENT     = 0b0010_0000;
        const ITERATION_STATEMENT     = 0b0100_0000;
        const JUMP_STATEMENT          = 0b1000_0000;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Keyword {
    Auto,
    Break,
    Case,
    Char,
    Const,
    Continue,
    Default,
    Do,
    Double,
    Else,
    Enum,
    Extern,
    Float,
    For,
    Goto,
    If,
    Inline,
    Int,
    Long,
    Register,
    Restrict,
    Return,
    Short,
    Signed,
    Sizeof,
    Static,
    Struct,
    Switch,
    Typedef,
    Union,
    Unsigned,
    Void,
    Volatile,
    While,
    Bool,
    Complex,
    Imaginary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FastKeyword {
    keyword: Keyword,
    category: KeywordCategory,
}

impl FastKeyword {
    pub fn from_str(s: &str) -> Option<Self> {
        KEYWORD_MAP
            .get(s).copied()
    }

    pub fn value(&self) -> Keyword {
        self.keyword
    }

    pub fn is_storage_class_specifier(&self) -> bool {
        self.category
            .contains(KeywordCategory::STORAGE_CLASS_SPECIFIER)
    }

    pub fn is_type_specifier(&self) -> bool {
        self.category.contains(KeywordCategory::TYPE_SPECIFIER)
    }

    pub fn is_simple_type_specifier(&self) -> bool {
        self.category
            .contains(KeywordCategory::SIMPLE_TYPE_SPECIFIER)
    }

    pub fn is_type_qualifier(&self) -> bool {
        self.category.contains(KeywordCategory::TYPE_QUALIFIER)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Punctuator {
    LSquare,
    RSquare,
    LParen,
    RParen,
    LCurly,
    RCurly,
    Dot,
    DashGreater,
    PlusPlus,
    MinusMinus,
    Ampersand,
    Star,
    Plus,
    Minus,
    BitNot,
    Not,
    Slash,
    Percent,
    LeftLeft,
    RightRight,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    EqualEqual,
    NotEqual,
    Carot,
    Bar,
    AmpersandAmpersand,
    BarBar,
    Question,
    Colon,
    SemiColon,
    ThreeDots,
    Equal,
    StarEqual,
    SlashEqual,
    PercentEqual,
    PlusEqual,
    MinusEqual,
    LeftEqual,
    RightEqual,
    AmpersandEqual,
    CarotEqual,
    BarEqual,
    Comma,
}

impl Punctuator {
    pub fn from_str(s: &str) -> Option<Punctuator> {
        PUNCTUATOR_MAP.get(s).copied()
    }
}

mod keyword_pairs {
    use super::Keyword::{self, *};
    use super::KeywordCategory;
    const STORAGE: KeywordCategory = KeywordCategory::STORAGE_CLASS_SPECIFIER;
    const TSPEC: KeywordCategory = KeywordCategory::TYPE_SPECIFIER;
    const SIMP_TSPEC: KeywordCategory = KeywordCategory::SIMPLE_TYPE_SPECIFIER;
    const TQUAL: KeywordCategory = KeywordCategory::TYPE_QUALIFIER;

    const LAB_STMT: KeywordCategory = KeywordCategory::LABELED_STATEMENT;
    const SEL_STMT: KeywordCategory = KeywordCategory::SELECTION_STATEMENT;
    const ITER_STMT: KeywordCategory = KeywordCategory::ITERATION_STATEMENT;
    const JUMP_STMT: KeywordCategory = KeywordCategory::JUMP_STATEMENT;

    const EMPTY: KeywordCategory = KeywordCategory::empty();

    pub static KEYWORD_PAIRS: &[(&str, (Keyword, KeywordCategory))] = &[
        ("auto", (Auto, STORAGE)),
        ("break", (Break, JUMP_STMT)),
        ("case", (Case, LAB_STMT)),
        ("char", (Char, SIMP_TSPEC)),
        ("const", (Const, TQUAL)),
        ("continue", (Continue, JUMP_STMT)),
        ("default", (Default, LAB_STMT)),
        ("do", (Do, ITER_STMT)),
        ("double", (Double, SIMP_TSPEC)),
        ("else", (Else, EMPTY)),
        ("enum", (Enum, TSPEC)),
        ("extern", (Extern, STORAGE)),
        ("float", (Float, SIMP_TSPEC)),
        ("for", (For, ITER_STMT)),
        ("goto", (Goto, JUMP_STMT)),
        ("if", (If, SEL_STMT)),
        ("inline", (Inline, EMPTY)),
        ("int", (Int, SIMP_TSPEC)),
        ("long", (Long, SIMP_TSPEC)),
        ("register", (Register, STORAGE)),
        ("restrict", (Restrict, TQUAL)),
        ("return", (Return, JUMP_STMT)),
        ("short", (Short, SIMP_TSPEC)),
        ("signed", (Signed, SIMP_TSPEC)),
        ("sizeof", (Sizeof, EMPTY)),
        ("static", (Static, STORAGE)),
        ("struct", (Struct, TSPEC)),
        ("switch", (Switch, SEL_STMT)),
        ("typedef", (Typedef, STORAGE)),
        ("union", (Union, TSPEC)),
        ("unsigned", (Unsigned, SIMP_TSPEC)),
        ("void", (Void, SIMP_TSPEC)),
        ("volatile", (Volatile, TQUAL)),
        ("while", (While, ITER_STMT)),
        ("_Bool", (Bool, SIMP_TSPEC)),
        ("_Complex", (Complex, SIMP_TSPEC)),
        ("_Imaginary", (Imaginary, SIMP_TSPEC)),
    ];
}

lazy_static! {
    static ref KEYWORD_MAP: HashMap<&'static str, FastKeyword> = {
        keyword_pairs::KEYWORD_PAIRS
            .iter()
            .copied()
            .map(|(name, (keyword, category))| (name, FastKeyword { keyword, category }))
            .collect()
    };
}

mod punctuator_pairs {
    use super::Punctuator::{self, *};

    pub static PUNCTUATOR_PAIRS: &[(&str, Punctuator)] = &[
        ("[", LSquare),
        ("]", RSquare),
        ("(", LParen),
        (")", RParen),
        ("{", LCurly),
        ("}", RCurly),
        (".", Dot),
        ("->", DashGreater),
        ("++", PlusPlus),
        ("--", MinusMinus),
        ("&", Ampersand),
        ("*", Star),
        ("+", Plus),
        ("-", Minus),
        ("~", BitNot),
        ("!", Not),
        ("/", Slash),
        ("%", Percent),
        ("<<", LeftLeft),
        (">>", RightRight),
        ("<", Less),
        (">", Greater),
        ("<=", LessEqual),
        (">=", GreaterEqual),
        ("==", EqualEqual),
        ("!=", NotEqual),
        ("^", Carot),
        ("|", Bar),
        ("&&", AmpersandAmpersand),
        ("||", BarBar),
        ("?", Question),
        (":", Colon),
        (";", SemiColon),
        ("...", ThreeDots),
        ("=", Equal),
        ("*=", StarEqual),
        ("/=", SlashEqual),
        ("%=", PercentEqual),
        ("+=", PlusEqual),
        ("-=", MinusEqual),
        ("<<=", LeftEqual),
        (">>=", RightEqual),
        ("&=", AmpersandEqual),
        ("^=", CarotEqual),
        ("|=", BarEqual),
        (",", Comma),
    ];
}

lazy_static! {
    static ref PUNCTUATOR_MAP: HashMap<&'static str, Punctuator> = {
        self::punctuator_pairs::PUNCTUATOR_PAIRS
            .iter()
            .copied()
            .collect()
    };
}
