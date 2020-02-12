// Copyright (C) 2019 - 2020 Jason Priest
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

//! Tokens encompassing strings of text used during preprocessing

use crate::front::location::Location;

/// The different kinds of [`PPToken`]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PPTokenKind {
    EndOfFile,

    Whitespace,
    Identifier,
    IdentifierNonExpandable,
    PPNumber,
    CharacterConstant,
    StringLiteral,
    Punctuator,
    Other,
}

/// A more complex token used in phases 3 and 4
///
/// Note: location is not considered in the PartialEq implementation
#[derive(Clone, Debug)]
pub struct PPToken {
    pub kind: PPTokenKind,
    pub value: String,
    pub location: Location,
}

impl PPToken {
    pub fn as_str(&self) -> &str {
        &*self.value
    }

    pub fn is_ident(&self) -> bool {
        self.kind == PPTokenKind::Identifier || self.kind == PPTokenKind::IdentifierNonExpandable
    }

    pub fn is_whitespace(&self) -> bool {
        self.kind == PPTokenKind::Whitespace
    }

    pub fn is_whitespace_not_newline(&self) -> bool {
        self.is_whitespace() && self.as_str() != "\n"
    }
}

// Static methods
impl PPToken {
    pub fn to_string(input: &[PPToken]) -> String {
        let mut output = String::new();

        for token in input {
            output.push_str(&token.value);
        }

        output
    }

    pub fn to_strings(input: &[PPToken]) -> Vec<&str> {
        input.iter().map(|t| t.as_str()).collect()
    }

    /// Compares two lists of [`PPTokens`](PPToken), ignoring whitespace.
    pub fn pptokens_loose_equal(a: &[PPToken], b: &[PPToken]) -> bool {
        let mut a = a.iter().filter(|t| !t.is_whitespace());
        let mut b = b.iter().filter(|t| !t.is_whitespace());

        loop {
            match (a.next(), b.next()) {
                // if elements match, continue
                (Some(a), Some(b)) if a == b => continue,

                // if both iterators terminate at same time, the lists were equal
                (None, None) => return true,

                // this covers the case where both iterators return Some but the
                // elements are different, as well as one iterator terminating early
                _ => return false,
            }
        }
    }

    /// Assert that two lists of [`PPTokens`](PPToken) are equal,
    /// ignoring whitespace.
    pub fn assert_loose_equal(a: &[PPToken], b: &[PPToken]) {
        let mut a = a.iter().enumerate().filter(|(_, t)| !t.is_whitespace());
        let mut b = b.iter().enumerate().filter(|(_, t)| !t.is_whitespace());

        loop {
            match (a.next(), b.next()) {
                // if elements match, continue
                (Some((_, a)), Some((_, b))) if a == b => continue,

                // if both iterators terminate at same time, the lists were equal
                (None, None) => return,

                // this covers the case where both iterators return Some but the
                // elements are different, as well as one iterator terminating early
                (Some((i1, a)), Some((i2, b))) => {
                    panic!(
                        "assertion failed: `(left == right)`\n  left[{}] = {:?}\n right[{}] = {:?}",
                        i1, a, i2, b
                    );
                }
                (Some((i1, a)), None) => {
                    panic!(
                        "assertion failed: `(left == right)`\n  left[{}] = {:?}\n right terminated",
                        i1, a
                    );
                }
                (None, Some((i2, b))) => {
                    panic!(
                        "assertion failed: `(left == right)`\n  left terminated\n right[{}] = {:?}",
                        i2, b
                    );
                }
            }
        }
    }
}

impl std::fmt::Display for PPToken {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Ignore location field
impl std::cmp::PartialEq for PPToken {
    fn eq(&self, rhs: &Self) -> bool {
        use PPTokenKind::*;
        match (self.kind, rhs.kind) {
            (EndOfFile, EndOfFile) => self.value == rhs.value,
            (Whitespace, Whitespace) => self.value == rhs.value,
            (Identifier, Identifier) => self.value == rhs.value,
            (IdentifierNonExpandable, IdentifierNonExpandable) => self.value == rhs.value,

            // identifiers can also compare to non-expandable identifiers
            (IdentifierNonExpandable, Identifier) => self.value == rhs.value,
            (Identifier, IdentifierNonExpandable) => self.value == rhs.value,

            (PPNumber, PPNumber) => self.value == rhs.value,
            (CharacterConstant, CharacterConstant) => self.value == rhs.value,
            (StringLiteral, StringLiteral) => self.value == rhs.value,
            (Punctuator, Punctuator) => self.value == rhs.value,
            (Other, Other) => self.value == rhs.value,
            _ => false,
        }
    }
}
