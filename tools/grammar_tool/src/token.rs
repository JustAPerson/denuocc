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

pub const EPSILON: &str = "\"\"";

pub fn string_set_crossproduct<'v, 'g: 'v>(
    lhs: impl IntoIterator<Item = &'v Vec<&'g str>> + Clone,
    rhs: impl IntoIterator<Item = &'v Vec<&'g str>> + Clone,
    limit: usize,
) -> HashSet<Vec<&'g str>> {
    // we will be reusing these iterators multiple times, hence why we require
    // the trait bound `lhs, rhs: Clone`
    debug_assert!(lhs.clone().into_iter().all(|v| v.len() <= limit));
    debug_assert!(rhs.clone().into_iter().all(|v| v.len() <= limit));

    if lhs.clone().into_iter().count() == 0 {
        rhs.into_iter()
            .map(|v| {
                v.iter()
                    .filter(|&&t| t != EPSILON)
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .collect()
    } else {
        let mut output = HashSet::new();
        for left in lhs {
            if left.iter().filter(|&&t| t != EPSILON).count() >= limit {
                output.insert(left.clone());
                continue;
            }
            for right in rhs.clone() {
                output.insert(
                    left.iter()
                        .chain(right.iter())
                        .filter(|&&t| t != EPSILON)
                        .take(limit)
                        .cloned()
                        .collect(),
                );
            }
        }
        output
    }
}
