// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! User visible messages about the input source code

use crate::core::{self, Severity};
use crate::front::c::minor::Encoding;
use crate::front::c::token::{PPTokenKind, TextPositionResolved, TokenOrigin};
use crate::front::c::tuctx::TUCtx;

/// Reusable element for [`MessageKind::ExpectedFound`][MessageKind::ExpectedFound]
#[derive(Clone, Debug)]
pub enum ExpectedFoundPart {
    Plain(String),
    PPToken(PPTokenKind),
    Directive(String),
}

impl std::fmt::Display for ExpectedFoundPart {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use ExpectedFoundPart::*;

        match self {
            Plain(string) => write!(f, "{}", string),
            PPToken(kind) => write!(f, "{} token", kind),
            Directive(directive) => write!(f, "`{}` directive", directive),
        }
    }
}

/// Type of a message
#[derive(Clone, Debug)]
pub enum MessageKind {
    ExpectedFound {
        expected: ExpectedFoundPart,
        found: ExpectedFoundPart,
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
    },
    Phase4MacroFirstDefined {
        name: String,
    },
    Phase4UndefineInvalidMacro {
        // TODO: Should be a very pedantic warning disabled by default
        name: String,
    },
    Phase4UnclosedMacroInvocation {
        name: String,
    },
    Phase4MacroInvocationOpening {
        name: String,
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
            Phase4MacroRedefinitionDifferent { name } => {
                format!("macro `{}` redefined differently", name,)
            },
            Phase4MacroFirstDefined { name } => format!("macro `{}` first defined here", name),
            Phase4UndefineInvalidMacro { name } => format!("macro `{}` does not exist", name),
            Phase4UnclosedMacroInvocation { name } => {
                format!("expected `)` to end invocation of macro `{}`", name,)
            },
            Phase4MacroInvocationOpening { name } => {
                format!("macro `{}` invocation opened here", name)
            },
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

    pub fn severity(&self) -> Severity {
        use MessageKind::*;
        match self {
            Phase4MacroInvocationOpening { .. } | Phase4MacroFirstDefined { .. } => Severity::Info,
            _ => Severity::Fatal, // TODO message severities
        }
    }
}

#[derive(Clone, Debug)]
pub struct Extra {
    pub enriched: String,
    pub position: TextPositionResolved,
}

/// A message about the source code being processed
#[derive(Clone, Debug)]
pub struct Message {
    pub kind: MessageKind,
    pub origin: TokenOrigin,
    pub children: Option<Box<[Message]>>,
    pub extra: Option<Extra>,
}

impl Message {
    pub fn enrich(&mut self, tuctx: &TUCtx) {
        use std::fmt::Write;

        let mut string = String::new();
        let span = self.origin.macro_root_textspan(tuctx);
        // let (name, lno, cno) = span.alias_line_column(tuctx);
        let textpos = span.pos.resolve(tuctx);

        writeln!(
            &mut string,
            "{}: {}",
            self.kind.severity(),
            self.kind.get_headline()
        )
        .unwrap();
        writeln!(&mut string, "  {}", textpos).unwrap();
        writeln!(&mut string, "  {}", span.text(tuctx)).unwrap();

        self.extra = Some(Extra {
            enriched: string,
            position: textpos.own_string(),
        });

        if let Some(children) = &mut self.children {
            children.iter_mut().for_each(|t| t.enrich(tuctx));
        }
    }

    pub fn enriched_message(&self) -> &String {
        self.extra
            .as_ref()
            .map(|t| &t.enriched)
            .expect("This message has not been enriched")
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(extra) = &self.extra {
            write!(f, "{}: {}", extra.position, self.kind.get_headline())
        } else {
            write!(f, "{}", self.kind.get_headline())
        }
    }
}

impl core::Message for Message {
    fn severity(&self) -> Option<Severity> {
        Some(self.kind.severity())
    }
}

impl std::convert::From<(TokenOrigin, MessageKind)> for Message {
    fn from(pair: (TokenOrigin, MessageKind)) -> Self {
        Message {
            kind: pair.1,
            origin: pair.0,
            children: None,
            extra: None,
        }
    }
}
