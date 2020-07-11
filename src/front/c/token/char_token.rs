// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Tokens encompassing a single character

use crate::front::c::input::Input;
use crate::front::c::token::{TextPosition, TextSpan};

/// A very simple token used in phases 1-3
#[derive(Clone, Copy, Debug)]
pub struct CharToken {
    pub value: char,
    pub span: TextSpan,
}

impl CharToken {
    pub fn from_input(input: &Input) -> Vec<CharToken> {
        Self::from_str(input.id, &input.content)
    }

    pub fn from_str(input: u32, content: &str) -> Vec<CharToken> {
        assert!((content.len() as u32) < u32::MAX);

        content
            .chars()
            .enumerate()
            .map(|(i, c)| CharToken {
                value: c,
                span: TextSpan {
                    pos: TextPosition {
                        input,
                        absolute: i as u32,
                    },
                    len: 1,
                },
            })
            .collect()
    }

    pub fn is_whitespace(&self) -> bool {
        [' ', '\n', '\t'].contains(&self.value)
    }

    /// Converts the given list of [`CharTokens`](CharToken) into a string.
    pub fn to_string(tokens: &[CharToken]) -> String {
        return tokens.iter().map(|t| t.value).collect();
    }
}

// Static methods
impl CharToken {
    /// Assert that two lists of [`CharTokens`](CharToken) are equal
    pub fn assert_equal(a: &[Self], b: &[Self]) {
        // let mut a = a.iter().enumerate().filter(|(_, t)| !t.is_whitespace());
        // let mut b = b.iter().enumerate().filter(|(_, t)| !t.is_whitespace());
        let mut a = a.iter().enumerate();
        let mut b = b.iter().enumerate();

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
                },
                (Some((i1, a)), None) => {
                    panic!(
                        "assertion failed: `(left == right)`\n  left[{}] = {:?}\n right terminated",
                        i1, a
                    );
                },
                (None, Some((i2, b))) => {
                    panic!(
                        "assertion failed: `(left == right)`\n  left terminated\n right[{}] = {:?}",
                        i2, b
                    );
                },
            }
        }
    }
}

// Ignore location field
impl std::cmp::PartialEq for CharToken {
    fn eq(&self, rhs: &Self) -> bool {
        // ignore location field
        self.value == rhs.value
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_chartokens_from_str() {
        let input = Input::new("<unit-test>".to_owned(), "abc\nd\ne".to_owned(), None);
        let tokens = CharToken::from_input(&input);

        assert_eq!(tokens[0].value, 'a');
        assert_eq!(tokens[0].span.pos.absolute, 0);
        assert_eq!(tokens[1].value, 'b');
        assert_eq!(tokens[1].span.pos.absolute, 1);
        assert_eq!(tokens[2].value, 'c');
        assert_eq!(tokens[2].span.pos.absolute, 2);
        assert_eq!(tokens[3].value, '\n');
        assert_eq!(tokens[3].span.pos.absolute, 3);
        assert_eq!(tokens[4].value, 'd');
        assert_eq!(tokens[4].span.pos.absolute, 4);
        assert_eq!(tokens[5].value, '\n');
        assert_eq!(tokens[5].span.pos.absolute, 5);
        assert_eq!(tokens[6].value, 'e');
        assert_eq!(tokens[6].span.pos.absolute, 6);
    }
}
