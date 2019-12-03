// Copyright (C) 2019 Jason Priest
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either  version 3 of the  License, or (at your  option) any later
// version.
//
// This program is distributed  in the hope that it will  be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR  A PARTICULAR  PURPOSE.  See  the GNU  General  Public  License for  more
// details.
//
// You should have received a copy of  the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

//! User visible messages about the input file

use crate::token::{Location, PPTokenKind};

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
    ExpectedFound {
        expected: MessagePart,
        found: MessagePart,
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
            Phase1FileEndingWithBackslash => write!(f, "file cannot end with a backslash"),
            Phase3MissingTerminator { terminator } => {
                write!(f, "missing closing {} terminator", terminator)
            }
            Phase4UnexpectedDirective { directive } => {
                write!(f, "unexpected directive `{}`", &directive)
            }
            Phase4InvalidDirective { directive } => write!(f, "invalid directive `{}`", &directive),
            Phase4DefineOperator => {
                write!(f, "expected identifier or left-paren after define operator")
            }
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
            ExpectedFound { expected, found } => {
                write!(f, "expected {}; found {}", expected, found)
            }
        }
    }
}
