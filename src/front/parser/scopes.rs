// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Scope tracking

// function, file, block, prototype
// label is function scope
// namespaces: labels, tags, typedefs, ordinary (variables, enum constants)

enum ScopeItem<T> {
    Present(T),
    /* Indirect(*const T),
     * Absent, */
}

use std::collections::HashMap;
type Namespace<T> = Vec<HashMap<String, ScopeItem<T>>>;
pub struct Scopes {
    labels: Namespace<()>,
    tags: Namespace<()>,
    typedefs: Namespace<()>,
    ordinarys: Namespace<()>,
}

impl Scopes {
    pub fn new() -> Self {
        Self {
            labels: Namespace::new(),
            tags: Namespace::new(),
            typedefs: Namespace::new(),
            ordinarys: Namespace::new(),
        }
    }

    fn namespace_contains<T>(namespace: &Namespace<T>, identifier: &str) -> bool {
        debug_assert!(!namespace.is_empty());
        for i in (0..namespace.len()).rev() {
            if namespace[i].contains_key(identifier) {
                return true;
            }
        }
        return false;
    }

    fn namespace_register<T>(namespace: &mut Namespace<T>, identifier: &str, value: T) {
        namespace.last_mut().unwrap().insert(identifier.to_owned(), ScopeItem::Present(value));
    }

    pub fn contains_typedef(&mut self, identifier: &str) -> bool {
        Self::namespace_contains(&self.typedefs, identifier)
    }

    pub fn register_typedef(&mut self, identifier: &str) {
        Self::namespace_register(&mut self.typedefs, identifier, ());
    }

    pub fn start_scope(&mut self) {
        self.labels.push(Default::default());
        self.tags.push(Default::default());
        self.typedefs.push(Default::default());
        self.ordinarys.push(Default::default());
    }

    pub fn end_scope(&mut self) {
        self.labels.pop();
        self.tags.pop();
        self.typedefs.pop();
        self.ordinarys.pop();
    }
}
