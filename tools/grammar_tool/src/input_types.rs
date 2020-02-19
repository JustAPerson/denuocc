// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

#[derive(Debug)]
pub struct Definition {
    pub name: String,
    pub alternates: Vec<Vec<Term>>,
}

#[derive(Debug)]
pub enum Term {
    String(String),
    Identifier(String),
}

impl AsRef<str> for Term {
    fn as_ref(&self) -> &str {
        match self {
            Term::String(s) => &s,
            Term::Identifier(s) => &s,
        }
    }
}
