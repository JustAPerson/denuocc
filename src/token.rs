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

//! Data structures used during lexing/parsing

use std::rc::Rc;

use crate::driver::Input;

/// A specific point in a file
#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub absolute: u32,
    pub line: u32,
    pub column: u32,
}

/// A region of code
#[derive(Clone, Debug)]
pub struct Location {
    pub input: Rc<Input>,
    pub begin: Position,
    pub len: u32,
}

impl<'a> std::ops::Sub<Location> for &'a Location {
    type Output = Location;
    fn sub(self, other: Location) -> Location {
        debug_assert!(Rc::ptr_eq(&self.input, &other.input));

        let end = self.begin.absolute + self.len;
        let begin = other.begin.absolute;

        debug_assert!(end > begin);
        let len = end - begin;

        Location {
            input: other.input,
            begin: other.begin,
            len: len,
        }
    }
}

impl Location {
    pub fn fmt_begin(&self) -> String {
        format!(
            "{}:{}:{}",
            self.input.name, self.begin.line, self.begin.column
        )
    }
}

/// A very simple token used in phases 1-3
#[derive(Clone, Debug)]
pub struct CharToken {
    pub value: char,
    pub loc: Location,
}

impl CharToken {
    /// Converts the given input into a list of [`CharTokens`].
    ///
    /// [`CharTokens`]: struct.CharToken.html
    pub fn from_input(input: &Rc<Input>) -> Vec<CharToken> {
        let mut output = Vec::new();

        let mut position = Position {
            absolute: 0,
            line: 1,
            column: 0,
        };

        for c in input.content.chars() {
            output.push(CharToken {
                value: c,
                loc: Location {
                    input: Rc::clone(input),
                    begin: position,
                    len: 1,
                },
            });

            // suffices for the other two counters
            position.absolute = position.absolute.checked_add(1).unwrap();

            position.column += 1;

            if c == '\n' {
                position.line += 1;
                position.column = 0;
            }
        }

        return output;
    }

    /// Converts the given list of [`CharTokens`] into a string.
    ///
    /// [`CharTokens`]: struct.CharToken.html
    pub fn to_string(tokens: &[CharToken]) -> String {
        return tokens.iter().map(|t| t.value).collect();
    }
}

/// The different kinds of [`PPToken`](./struct.PPToken.html)
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PPTokenKind {
    Placemarker,
    EndOfFile,

    Whitespace,
    HeaderName,
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
    // pub fn as_str(&self) -> &str {
    //     let begin = self.location.begin.absolute as usize;
    //     let len = self.location.len as usize;
    //     &self.location.input.content[begin..begin + len]
    // }

    pub fn as_str(&self) -> &str {
        &*self.value
    }

    pub fn to_string(input: &[PPToken]) -> String {
        let mut output = String::new();

        for token in input {
            output.push_str(&token.value);
        }

        output
    }

    pub fn is_whitespace(&self) -> bool {
        self.kind == PPTokenKind::Whitespace
    }
}

// Ignore location field
impl std::cmp::PartialEq for PPToken {
    fn eq(&self, rhs: &Self) -> bool {
        use PPTokenKind::*;
        match (self.kind, rhs.kind) {
            (Placemarker, Placemarker) => self.value == rhs.value,
            (EndOfFile, EndOfFile) => self.value == rhs.value,
            (Whitespace, Whitespace) => self.value == rhs.value,
            (HeaderName, HeaderName) => self.value == rhs.value,
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

pub fn assert_pptokens_loose_equal(a: &[PPToken], b: &[PPToken]) {
    let mut a = a.iter().enumerate().filter(|(_, t)| !t.is_whitespace());
    let mut b = b.iter().enumerate().filter(|(_, t)| !t.is_whitespace());

    loop {
        match dbg!((a.next(), b.next())) {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_location_sub() {
        let input = Rc::new(Input {
            name: "<unit-test>".to_owned(),
            content: "abc\nd\ne".to_owned(),
            is_file: false,
        });
        let tokens = CharToken::from_input(&input);
        let diff = &tokens[2].loc - tokens[0].loc.clone();

        assert!(Rc::ptr_eq(&input, &diff.input));
        assert_eq!(diff.begin.absolute, 0);
        assert_eq!(diff.begin.line, 1);
        assert_eq!(diff.begin.column, 0);
        assert_eq!(diff.len, 3);
    }

    #[test]
    #[should_panic(expected = "assertion failed: end > begin")]
    fn test_location_sub_backwards() {
        let input = Rc::new(Input {
            name: "<unit-test>".to_owned(),
            content: "abc\nd\ne".to_owned(),
            is_file: false,
        });
        let tokens = CharToken::from_input(&input);
        let _ = &tokens[0].loc - tokens[2].loc.clone();
    }

    // #[test]
    // #[should_panic(expected = "assertion failed: `(left == right)`")]
    // fn test_location_same_line() {
    //     let input = Rc::new(Input {
    //         name: "<unit-test>".to_owned(),
    //         content: "abc\nd\ne".to_owned(),
    //         is_file: false,
    //     });
    //     let tokens = chartokens_from_input(&input);
    //     let _ = &tokens[5].loc - tokens[0].loc.clone();
    // }

    #[test]
    fn test_chartokens_from_str() {
        let input = Rc::new(Input {
            name: "<unit-test>".to_owned(),
            content: "abc\nd\ne".to_owned(),
            is_file: false,
        });
        let tokens = CharToken::from_input(&input);

        assert_eq!(tokens[0].value, 'a');
        assert_eq!(tokens[0].loc.begin.absolute, 0);
        assert_eq!(tokens[0].loc.begin.line, 1);
        assert_eq!(tokens[0].loc.begin.column, 0);

        assert_eq!(tokens[1].value, 'b');
        assert_eq!(tokens[1].loc.begin.absolute, 1);
        assert_eq!(tokens[1].loc.begin.line, 1);
        assert_eq!(tokens[1].loc.begin.column, 1);

        assert_eq!(tokens[2].value, 'c');
        assert_eq!(tokens[2].loc.begin.absolute, 2);
        assert_eq!(tokens[2].loc.begin.line, 1);
        assert_eq!(tokens[2].loc.begin.column, 2);

        assert_eq!(tokens[3].value, '\n');
        assert_eq!(tokens[3].loc.begin.absolute, 3);
        assert_eq!(tokens[3].loc.begin.line, 1);
        assert_eq!(tokens[3].loc.begin.column, 3);

        assert_eq!(tokens[4].value, 'd');
        assert_eq!(tokens[4].loc.begin.absolute, 4);
        assert_eq!(tokens[4].loc.begin.line, 2);
        assert_eq!(tokens[4].loc.begin.column, 0);

        assert_eq!(tokens[5].value, '\n');
        assert_eq!(tokens[5].loc.begin.absolute, 5);
        assert_eq!(tokens[5].loc.begin.line, 2);
        assert_eq!(tokens[5].loc.begin.column, 1);

        assert_eq!(tokens[6].value, 'e');
        assert_eq!(tokens[6].loc.begin.absolute, 6);
        assert_eq!(tokens[6].loc.begin.line, 3);
        assert_eq!(tokens[6].loc.begin.column, 0);
    }
}
