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

//! Represents a location within the input text

use std::rc::Rc;

use crate::driver::Input;

/// A specific point in a file
#[derive(Copy, Clone, Debug, Default)]
pub struct Position {
    pub absolute: u32,
    pub line: u32,
    pub column: u32,
}

/// A region of code
#[derive(Clone)]
pub struct DirectLocation {
    pub input: Rc<Input>,
    pub begin: Position,
    pub len: u32,
}

impl<'a> std::ops::Sub<DirectLocation> for &'a DirectLocation {
    type Output = DirectLocation;
    fn sub(self, other: DirectLocation) -> DirectLocation {
        debug_assert!(Rc::ptr_eq(&self.input, &other.input));

        let end = self.begin.absolute + self.len;
        let begin = other.begin.absolute;

        debug_assert!(end > begin);
        let len = end - begin;

        DirectLocation {
            input: other.input,
            begin: other.begin,
            len: len,
        }
    }
}

impl std::fmt::Debug for DirectLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.fmt_begin())
    }
}

impl DirectLocation {
    /// Returns a string with filename, line number, and column in a canonical
    /// pattern
    pub fn fmt_begin(&self) -> String {
        format!(
            "{}:{}:{}",
            self.input.name, self.begin.line, self.begin.column
        )
    }
}

#[derive(Clone)]
pub enum Location {
    Direct(DirectLocation),
    Indirect(Vec<DirectLocation>),
}

impl std::fmt::Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.fmt_begin())
    }
}

impl std::convert::From<DirectLocation> for Location {
    fn from(loc: DirectLocation) -> Location {
        Location::Direct(loc)
    }
}

impl Location {
    /// Returns a string with filename, line number, and column in a canonical
    /// pattern
    pub fn fmt_begin(&self) -> String {
        match self {
            Location::Direct(loc) => loc.fmt_begin(),
            Location::Indirect(locs) => locs.last().unwrap().fmt_begin(),
        }
    }

    /// Push a new location to signify where this token came from
    ///
    /// This will occur when expanding macros or including files
    pub fn push(&mut self, new_loc: DirectLocation) {
        match self {
            Location::Direct(old_loc) => *self = Location::Indirect(vec![old_loc.clone(), new_loc]),
            Location::Indirect(locs) => locs.push(new_loc),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::front::token::CharToken;

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
}
