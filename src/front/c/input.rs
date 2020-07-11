// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Input source code for compilation

use std::path::PathBuf;
use std::rc::Rc;

use crate::front::c::token::TextSpan;
use crate::util::Hashed;

/// Represents how a file was included by the preprocessor
#[derive(Clone, Debug)]
pub struct IncludedFrom {
    /// The input that performed the inclusion (not the one that was
    /// included)
    pub input: Rc<Input>,

    /// The entire `#include` line
    pub span: TextSpan,
}

/// An input to the compilation process
#[derive(Clone, Debug)]
pub struct Input {
    pub name: String,
    pub content: Hashed<String>,
    pub path: Option<PathBuf>,
    pub included_from: Option<IncludedFrom>,
    pub depth: usize,
    pub id: u32,
    newlines: Vec<u32>,
}

impl Input {
    pub fn new(name: String, content: String, path: Option<PathBuf>) -> Self {
        // TODO make this an Option generated on demand
        let newlines = content
            .chars()
            .enumerate()
            .filter(|&(_, c)| c == '\n')
            .map(|(i, _)| i as u32)
            .collect();
        let content = Hashed::new(content);
        Self {
            name,
            content,
            path,
            included_from: None,
            depth: 0,
            id: 0,
            newlines,
        }
    }

    pub fn get_line_column(&self, absolute: u32) -> (u32, u32) {
        let len = self.newlines.len();
        let (line, column) = match self.newlines.binary_search(&absolute) {
            Ok(0) => (1, self.newlines[0] + 1),
            Ok(i) => (i as u32 + 1, self.newlines[i] - self.newlines[i - 1]),
            Err(0) => (1, absolute + 1),
            Err(i) if i < len => (i as u32 + 1, absolute - self.newlines[i - 1]),
            Err(_) => (len as u32 + 1, absolute - self.newlines.last().unwrap()),
        };

        // all existing test cases are written with the assumption that column
        // number starts at zero. I got that convention from emacs, but seems
        // most compilers do not use it. I'll be fixing this in the next commit.
        (line, column - 1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_input_line_column() {
        const STRING: &'static str = "\
        abc\ndef\ng";

        fn calc(c: char) -> (u32, u32) {
            let absolute = STRING.find(c).unwrap() as u32;
            let result =
                Input::new("".to_owned(), STRING.to_owned(), None).get_line_column(absolute);
            println!(
                "char(c = {:?}) absolute = {} result = {:?}",
                c, absolute, result
            );
            result
        }

        assert_eq!(calc('a'), (1, 1));
        assert_eq!(calc('b'), (1, 2));
        assert_eq!(calc('c'), (1, 3));
        assert_eq!(calc('d'), (2, 1));
        assert_eq!(calc('e'), (2, 2));
        assert_eq!(calc('f'), (2, 3));
        assert_eq!(calc('g'), (3, 1));
    }
}
