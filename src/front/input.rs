// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

use std::rc::Rc;
use std::path::PathBuf;

use crate::front::location::Position;

#[derive(Clone)]
pub struct IncludedFrom {
    /// The input that performed the inclusion (not the one that was
    /// included)
    input: Rc<Input>,
    /// The line where the inclusion was performed
    position: Position,
    /// The length of the including line
    len: u32,
}

/// An input to the compilation process
#[derive(Clone)]
pub struct Input {
    pub name: String,
    pub content: String,
    pub path: Option<PathBuf>,
    pub included_from: Option<IncludedFrom>,
}

impl Input {
    pub fn new(name: String, content: String, path: Option<PathBuf>) -> Self {
        Self {
            name,
            content,
            path,
            included_from: None,
        }
    }
}

impl std::fmt::Debug for IncludedFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Avoid printing entire inclusion chain
        f.debug_struct("IncludedFrom")
            .field("input_name", &self.input.name)
            .field("position", &self.position)
            .field("len", &self.len)
            .finish()
    }
}

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Avoid printing the entire `content` field
        let content_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            self.content.hash(&mut hasher);
            hasher.finish()
        };

        f.debug_struct("Input")
            .field("name", &self.name)
            .field("content_hash", &content_hash)
            .field("path", &self.path)
            .field("included", &self.included_from)
            .finish()
    }
}
