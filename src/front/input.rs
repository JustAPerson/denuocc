// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Input source code for compilation

use std::path::PathBuf;
use std::rc::Rc;

use crate::front::location::Span;
use crate::util::Hashed;

/// Represents how a file was included by the preprocessor
#[derive(Clone)]
pub struct IncludedFrom {
    /// The input that performed the inclusion (not the one that was
    /// included)
    pub input: Rc<Input>,
    pub span: Span,
}

/// An input to the compilation process
#[derive(Clone, Debug)]
pub struct Input {
    pub name: String,
    pub content: Hashed<String>,
    pub path: Option<PathBuf>,
    pub included_from: Option<IncludedFrom>,
    pub depth: usize,
    pub tu_id: u16,
}

impl Input {
    pub fn new(name: String, content: String, path: Option<PathBuf>) -> Self {
        let content = Hashed::new(content);
        Self {
            name,
            content,
            path,
            included_from: None,
            depth: 0,
            tu_id: 0,
        }
    }
}

impl std::fmt::Debug for IncludedFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Avoid printing entire inclusion chain
        f.debug_struct("IncludedFrom")
            .field("input_name", &self.input.name)
            .field("span_begin_fmt", &self.span.begin.fmt_begin())
            .finish()
    }
}
