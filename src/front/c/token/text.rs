// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Textual locations within the input source code

use std::rc::Rc;

use crate::front::c::input::Input;
use crate::front::c::tuctx::TUCtx;

/// A resolved version of [`TextPosition`][TextPosition]
#[derive(Clone, Copy, Debug)]
pub struct TextPositionResolved<T = String>
where
    T: std::fmt::Display,
{
    input: T,
    line: u32,
    column: u32,
}

impl<T: std::fmt::Display> std::fmt::Display for TextPositionResolved<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.input, self.line, self.column)
    }
}

impl TextPositionResolved<&str> {
    // could not figure out how to implement ToOwned
    pub fn own_string(&self) -> TextPositionResolved<String> {
        TextPositionResolved {
            input: self.input.to_owned(),
            line: self.line,
            column: self.column,
        }
    }
}

/// An exact position in the source code
#[derive(Clone, Copy, Debug)]
pub struct TextPosition {
    pub input: u32,
    pub absolute: u32,
}

impl TextPosition {
    pub(crate) fn input<'a>(&self, tuctx: &'a TUCtx) -> &'a Rc<Input> {
        &tuctx.inputs[self.input as usize]
    }

    pub fn resolve<'a>(&self, tuctx: &'a TUCtx) -> TextPositionResolved<&'a str> {
        let input = self.input(tuctx);
        let (line, column) = input.get_line_column(self.absolute);
        TextPositionResolved {
            input: &input.name,
            line,
            column,
        }
    }
}

/// A region of text in the source code
// Note, we could use a u16 here, and reduce TextPosition::input to u16 as
// well. Thus, this whole structure would only occupy 8 bytes.
//
// The standard only requires tokens up to 4KiB (ISO 9899:2018 5.2.4.1).
#[derive(Clone, Copy, Debug)]
pub struct TextSpan {
    pub pos: TextPosition,
    pub len: u32,
}

impl std::ops::Deref for TextSpan {
    type Target = TextPosition;
    fn deref(&self) -> &Self::Target {
        &self.pos
    }
}

impl TextSpan {
    pub fn between(low: &TextPosition, high: &TextPosition) -> TextSpan {
        debug_assert!(low.input == high.input);
        TextSpan {
            pos: *low,
            len: high.absolute - low.absolute,
        }
    }

    pub fn text<'a>(&self, tuctx: &'a TUCtx) -> &'a str {
        let beg = self.pos.absolute as usize;
        let end = beg + (self.len as usize);
        &self.pos.input(tuctx).content[beg..end]
    }

    pub fn begin(&self) -> TextPosition {
        self.pos
    }

    pub fn end(&self) -> TextPosition {
        let mut pos = self.pos;
        pos.absolute += self.len;
        pos
    }
}
