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

use std::collections::HashMap;

use crate::first::First;
use crate::grammar::Grammar;
use crate::token::StringSet;

pub struct Follow<'g> {
    sets: HashMap<&'g str, StringSet<'g>>,
}

impl<'g> Follow<'g> {
    pub fn new(grammar: &'g Grammar, first: &'g First<'g>) -> Follow<'g> {
        let mut sets = HashMap::<&str, StringSet>::new();
        for nonterminal in &grammar.nonterminals {
            let mut set = StringSet::new();
            for production in &grammar.production_map[nonterminal] {
                for i in 0..production.tokens.len() {
                    if production.tokens[i] == *nonterminal {
                        set.extend(first.query_string(&production.tokens[i..]));
                    }
                }
            }
            sets.insert(nonterminal, set);
        }

        Follow { sets }
    }

    pub fn query_token(&self, token: &str) -> &StringSet<'g> {
        &self.sets[token]
    }
}
