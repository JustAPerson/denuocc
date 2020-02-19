// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

use std::collections::HashSet;

pub type StringSet<'g> = HashSet<Vec<&'g str>>;

pub fn empty_string_set() -> StringSet<'static> {
    let mut set = StringSet::new();
    set.insert(Vec::new());
    set
}

pub fn string_set_crossproduct<'v, 'g: 'v>(
    lhs: impl IntoIterator<Item = &'v Vec<&'g str>> + Clone,
    rhs: impl IntoIterator<Item = &'v Vec<&'g str>> + Clone,
    limit: usize,
) -> HashSet<Vec<&'g str>> {
    // we will be reusing these iterators multiple times, hence why we require
    // the trait bound `lhs, rhs: Clone`
    debug_assert!(lhs.clone().into_iter().all(|v| v.len() <= limit));
    debug_assert!(rhs.clone().into_iter().all(|v| v.len() <= limit));

    let mut output = HashSet::new();
    for left in lhs {
        if left.len() >= limit {
            output.insert(left.clone());
            continue;
        }
        for right in rhs.clone() {
            output.insert(
                left.iter()
                    .chain(right.iter())
                    .take(limit)
                    .cloned()
                    .collect(),
            );
        }
    }
    output
}
