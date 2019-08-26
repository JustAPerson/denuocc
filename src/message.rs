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
    Phase4MacroType {
        name: String,
        defined: &'static str,
        used: &'static str,
    },
    Phase4MacroArity {
        name: String,
        expected: usize,
        found: usize,
    },
    Phase4MacroArityVararg {
        name: String,
        expected: usize,
        found: usize,
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
    pub include_history: Vec<(String, u32)>,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use MessageKind::*;

        for (name, line) in &self.include_history {
            writeln!(f, "Included from {}:{}:", &name, line)?;
        }
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
            Phase4MacroType {
                name,
                defined,
                used,
            } => write!(
                f,
                "`{}` was defined as {} macro but is being used as {} macro",
                name, defined, used
            ),
            Phase4MacroArity {
                name,
                expected,
                found,
            } => write!(
                f,
                "`{}` expects exactly {} arguments; found {}",
                name, expected, found
            ),
            Phase4MacroArityVararg {
                name,
                expected,
                found,
            } => write!(
                f,
                "`{}` expects at least {} arguments; found {}",
                name, expected, found
            ),
            ExpectedFound { expected, found } => {
                write!(f, "expected {}; found {}", expected, found)
            }
        }
    }
}
