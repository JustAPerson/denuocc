// Copyright (C) 2020 Jason Priest
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
