// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! User visible messages about the input file

use crate::front;
use crate::front::location::Location;
use crate::front::minor::Encoding;
use crate::front::token::PPTokenKind;
use crate::front::types;

#[derive(Copy, Clone, Debug)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub enum MessagePart {
    Plain(String),
    PPToken(PPTokenKind),
    Directive(String),
}

impl std::fmt::Display for MessagePart {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use MessagePart::*;

        match self {
            Plain(string) => write!(f, "{}", string),
            PPToken(kind) => write!(f, "`{:?}` token", kind),
            Directive(directive) => write!(f, "`{}` directive", directive),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MessageKind {
    ExpectedFound {
        expected: MessagePart,
        found: MessagePart,
    },
    Phase1FileEndingWithBackslash,
    Phase3MissingTerminator {
        terminator: char,
    },
    Phase4UnexpectedDirective {
        directive: String,
    },
    Phase4InvalidDirective {
        directive: String,
    },
    Phase4DefineOperator,
    Phase4MacroArity {
        name: String,
        expected: usize,
        found: usize,
        vararg: bool,
    },
    Phase4MacroRedefinitionDifferent {
        name: String,
        original: Location,
    },
    Phase4UndefineInvalidMacro {
        // TODO: Should be a very pedantic warning disabled by default
        name: String,
    },
    Phase4UnclosedMacroInvocation {
        name: String,
        open: Location,
    },
    Phase4RepeatedMacroParameter {
        parameter: String,
    },
    Phase4IllegalSingleHash,
    Phase4IllegalDoubleHash,
    Phase4BadConcatenation {
        lhs: String,
        rhs: String,
    },
    Phase5Empty,
    Phase5OutOfRange {
        prefix: &'static str,
        value: String,
        encoding: Encoding,
    },
    Phase5Invalid {
        prefix: &'static str,
        value: String,
    },
    Phase5Incomplete {
        expected: usize,
        found: usize,
        prefix: char,
    },
    Phase5Unrecognized {
        escape: char,
    },
    Phase6IncompatibleEncoding {
        previous: Encoding,
        current: Encoding,
    },
    Phase7UnrecognizedCharacter {
        character: char,
    },
    // Phase7InvalidNumber {
    //     value: String,
    //     reason: &'static str,
    // },
    Phase7FloatNoSignificand {
        radix: front::realize::Radix,
    },
    Phase7FloatNoExponent,
    Phase7NumberSuffixRepeat,
    Phase7NumberSuffixUnknown {
        suffix: String,
        radix: front::realize::Radix,
        numkind: front::realize::NumberKind,
    },
    Phase7NumberSuffixConflict {
        radix: front::realize::Radix,
        numkind: front::realize::NumberKind,
    },
    Phase7HexFloatUnrepresentable {
        value: String,
        datatype: types::FloatType,
    },
    Phase7DecFloatFailure {
        value: String,
        datatype: types::FloatType,
    },
    Phase7IntTooLarge {
        value: String,
    },
}

#[derive(Clone, Debug)]
pub struct Message {
    pub kind: MessageKind,
    pub location: Location,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use MessageKind::*;

        // TODO include history / macro expansion
        // for (name, line) in &self.include_history {
        //     writeln!(f, "Included from {}:{}:", &name, line)?;
        // }
        write!(f, "{}: ", self.location.fmt_begin())?;
        match &self.kind {
            ExpectedFound { expected, found } => {
                write!(f, "expected {}; found {}", expected, found)
            },
            Phase1FileEndingWithBackslash => write!(f, "file cannot end with a backslash"),
            Phase3MissingTerminator { terminator } => {
                write!(f, "missing closing {} terminator", terminator)
            },
            Phase4UnexpectedDirective { directive } => {
                write!(f, "unexpected directive `{}`", &directive)
            },
            Phase4InvalidDirective { directive } => write!(f, "invalid directive `{}`", &directive),
            Phase4DefineOperator => {
                write!(f, "expected identifier or left-paren after define operator")
            },
            Phase4MacroArity {
                name,
                expected,
                found,
                vararg,
            } => write!(
                f,
                "`{}` expects {} {} {}; found {}",
                name,
                if *vararg { "at least" } else { "exactly" },
                expected,
                if *expected == 1 {
                    "argument"
                } else {
                    "arguments"
                },
                found
            ),
            Phase4MacroRedefinitionDifferent { name, original } => write!(
                f,
                "macro `{}` was originally defined differently here: {}",
                name,
                original.fmt_begin(),
            ),
            Phase4UndefineInvalidMacro { name } => write!(f, "macro `{}` does not exist", name),
            Phase4UnclosedMacroInvocation { name, open } => write!(
                f,
                "expected `)` to end invocation of macro `{}` which opened at: {}",
                name,
                open.fmt_begin()
            ),
            Phase4RepeatedMacroParameter { parameter } => {
                write!(f, "macro parameter `{}` repeated", parameter)
            },
            Phase4IllegalSingleHash => {
                write!(f, "the `#` operator must be followed by a macro parameter")
            },
            Phase4IllegalDoubleHash => write!(f, "a macro cannot begin nor end with `##`"),
            Phase4BadConcatenation { lhs, rhs } => write!(
                f,
                "concatenating `{}` and `{}` does not result in a valid preprocessor token",
                lhs, rhs
            ),
            Phase5Empty => write!(f, "expected character after escape sequence"),
            Phase5Incomplete {
                expected,
                found,
                prefix,
            } => write!(
                f,
                "expected {} digits after `\\{}`; found {}",
                expected, prefix, found
            ),
            Phase5OutOfRange {
                prefix,
                value,
                encoding,
            } => write!(
                f,
                "`\\{}{}` exceeds range of type ({})",
                prefix,
                value,
                encoding.type_str()
            ),
            Phase5Invalid { prefix, value } => {
                write!(f, "`\\{}{}` cannot be represented", prefix, value)
            },
            Phase5Unrecognized { escape } => write!(f, "`\\{}` is not a valid escape", escape),
            Phase6IncompatibleEncoding { previous, current } => write!(
                f,
                "incompatible encoding when concatenating; previously `{}` but found `{}`",
                previous.to_str(),
                current.to_str()
            ),
            Phase7UnrecognizedCharacter { character } => write!(
                f,
                "unrecognized character `{}` (U+{:04X}); skipping",
                character, *character as usize
            ),
            // Phase7InvalidNumber { value, reason } => {
            //     write!(f, "invalid number constant `{}`: {}", value, reason)
            // }
            Phase7FloatNoSignificand { radix } => {
                write!(f, "{} float constant requires a significand", radix)
            }
            Phase7FloatNoExponent => write!(f, "hexadecimal float constant requires an exponent"),
            Phase7NumberSuffixRepeat => write!(f, "number type suffix cannot be repeated"),
            Phase7NumberSuffixUnknown {
                suffix,
                radix,
                numkind,
            } => write!(
                f,
                "unknown suffix `{}` for {} {} constant",
                suffix, radix, numkind
            ),
            Phase7NumberSuffixConflict { radix, numkind } => write!(
                f,
                "conflicting suffix combination for {} {} constant",
                radix, numkind
            ),
            Phase7HexFloatUnrepresentable { value, datatype } => write!(
                f,
                "hexadecimal float constant `{}` cannot accurately be represented as `{}`",
                value, datatype
            ),
            Phase7DecFloatFailure { value, datatype } => write!(
                f,
                "decimal float constant `{}` failed to parse as `{}`",
                value, datatype
            ),
            Phase7IntTooLarge { value } => write!(
                f,
                "integer literal is too large to be represented in any type"
            ),
        }
    }
}
