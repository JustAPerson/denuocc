// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! User visible messages about the input file

use crate::front::location::Location;
use crate::front::minor::Encoding;
use crate::front::token::PPTokenKind;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
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
            PPToken(kind) => write!(f, "{} token", kind),
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
    Phase4IncludeBegin,
    Phase4IncludeUnclosed,
    Phase4IncludeExtra {
        kind: PPTokenKind,
    },
    Phase4IncludeDepth,
    Phase4IncludeNotFound {
        desired_file: String,
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
}

impl MessageKind {
    /// Formats the message headline
    ///
    /// The headline conveys the summary of the message. When presenting to the
    /// end user, the message should be enriched with extra information.
    pub fn get_headline(&self) -> String {
        use MessageKind::*;
        match &self {
            ExpectedFound { expected, found } => format!("expected {}; found {}", expected, found),
            Phase1FileEndingWithBackslash => format!("file cannot end with a backslash"),
            Phase3MissingTerminator { terminator } => {
                format!("missing closing {} terminator", terminator)
            },
            Phase4UnexpectedDirective { directive } => {
                format!("unexpected directive `{}`", &directive)
            },
            Phase4InvalidDirective { directive } => format!("invalid directive `{}`", &directive),
            Phase4DefineOperator => {
                format!("expected identifier or left-paren after define operator")
            },
            Phase4MacroArity {
                name,
                expected,
                found,
                vararg,
            } => format!(
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
            Phase4MacroRedefinitionDifferent { name, original } => format!(
                "macro `{}` was originally defined differently here: {}",
                name,
                original.fmt_begin(),
            ),
            Phase4UndefineInvalidMacro { name } => format!("macro `{}` does not exist", name),
            Phase4UnclosedMacroInvocation { name, open } => format!(
                "expected `)` to end invocation of macro `{}` which opened at: {}",
                name,
                open.fmt_begin()
            ),
            Phase4RepeatedMacroParameter { parameter } => {
                format!("macro parameter `{}` repeated", parameter)
            },
            Phase4IllegalSingleHash => {
                format!("the `#` operator must be followed by a macro parameter")
            },
            Phase4IllegalDoubleHash => format!("a macro cannot begin nor end with `##`"),
            Phase4BadConcatenation { lhs, rhs } => format!(
                "concatenating `{}` and `{}` does not result in a valid preprocessor token",
                lhs, rhs
            ),
            Phase4IncludeBegin => format!(
                r#"expected `<FILENAME>`, `"FILENAME"`, or a macro that expands to either of those"#
            ),
            Phase4IncludeUnclosed => {
                format!("expected `>` to close corresponding `<` after `#include`",)
            },
            Phase4IncludeExtra { kind } => {
                format!("expected newline after <FILENAME>; found {}", kind)
            },
            Phase4IncludeDepth => format!("maximum nested include depth exceeded"),
            Phase4IncludeNotFound { desired_file } => {
                format!("could not include `{}`: file not found", desired_file)
            },
            Phase5Empty => format!("expected character after escape sequence"),
            Phase5Incomplete {
                expected,
                found,
                prefix,
            } => format!(
                "expected {} digits after `\\{}`; found {}",
                expected, prefix, found
            ),
            Phase5OutOfRange {
                prefix,
                value,
                encoding,
            } => format!(
                "`\\{}{}` exceeds range of type ({})",
                prefix,
                value,
                encoding.type_str()
            ),
            Phase5Invalid { prefix, value } => {
                format!("`\\{}{}` cannot be represented", prefix, value)
            },
            Phase5Unrecognized { escape } => format!("`\\{}` is not a valid escape", escape),
            Phase6IncompatibleEncoding { previous, current } => format!(
                "incompatible encoding when concatenating; previously `{}` but found `{}`",
                previous.to_str(),
                current.to_str()
            ),
        }
    }

    pub fn get_severity(&self) -> Severity {
        use Severity::*;
        match self {
            _ => Error, // TODO message severities
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub kind: MessageKind,
    pub location: Location,
}

impl Message {
    pub fn fmt_enriched_message(&self, output: &mut String) -> std::fmt::Result {
        use std::fmt::Write;
        writeln!(
            output,
            "{}: {}",
            self.kind.get_severity(),
            self.kind.get_headline()
        )?;
        writeln!(output, "  {}", self.location.fmt_begin())?;
        writeln!(
            output,
            "  {}",
            self.location.get_outermost_macro_use_begin().fmt_begin()
        )?;
        Ok(())
    }

    pub fn enriched_message(&self) -> String {
        let mut output = String::new();
        self.fmt_enriched_message(&mut output).unwrap();
        output
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}: ",
            self.location.get_outermost_macro_use_begin().fmt_begin()
        )?;
        write!(f, "{}", self.kind.get_headline())
    }
}
