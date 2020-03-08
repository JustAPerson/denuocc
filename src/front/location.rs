// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Represents a location within the input text

use std::rc::Rc;

use crate::front::input::Input;
use crate::front::preprocessor::MacroDef;

/// A specific point in a file
#[derive(Copy, Clone, Debug, Default)]
pub struct Position {
    pub absolute: u32,
    // TODO store absolute position of each newline character in a sorted array
    // and use binary search to convert an absolute position to a line number
    // and column.
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug)]
pub struct MacroUse {
    pub macro_definition: Rc<MacroDef>,
    pub location: Location,
}

/// Where a token came from
///
/// This represents the file
#[derive(Clone, Debug)]
pub struct Location {
    pub begin: Position,
    pub len: u32,
    pub macro_use: Option<Rc<MacroUse>>,
    pub input: Rc<Input>,
}

impl Location {
    /// Returns a string with filename, line number, and column in a canonical
    /// pattern
    pub fn fmt_begin(&self) -> String {
        format!(
            "{}:{}:{}",
            self.input.name, self.begin.line, self.begin.column
        )
    }
}

#[derive(Clone, Debug)]
pub struct Span {
    pub begin: Location,
    pub end: Location,
}

impl Span {
    pub fn new(begin: Location, end: Location) -> Span {
        Span { begin, end }
    }
}

#[derive(Clone, Debug)]
pub struct Spanned<T> {
    pub span: Span,
    pub value: T,
}

impl<T> Spanned<T> {
    pub fn map<U, F>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned {
            span: self.span,
            value: f(self.value),
        }
    }

    pub fn into<U: From<T>>(self) -> Spanned<U> {
        Spanned {
            span: self.span,
            value: self.value.into(),
        }
    }
}
