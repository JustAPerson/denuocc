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
        Builder::new(grammar, first).build()
    }

    pub fn query_token(&self, token: &str) -> &StringSet<'g> {
        &self.sets[token]
    }
}

struct Builder<'g> {
    grammar: &'g Grammar,
    first: &'g First<'g>,
    sets: HashMap<&'g str, StringSet<'g>>,
}

impl<'g> Builder<'g> {
    pub fn new(grammar: &'g Grammar, first: &'g First<'g>) -> Builder<'g> {
        Builder {
            grammar,
            first,
            sets: HashMap::new(),
        }
    }

    fn build(mut self) -> Follow<'g> {
        self.build_normal();
        self.build_sentential_tails(&self.grammar.start);

        Follow { sets: self.sets }
    }

    fn build_normal(&mut self) {
        for nonterminal in &self.grammar.nonterminals {
            let mut set = StringSet::new();
            for production in &self.grammar.productions {
                for i in 0..production.tokens.len() {
                    if production.tokens[i] == *nonterminal {
                        set.extend(self.first.query_string(&production.tokens[i + 1..]));
                    }
                }
            }
            self.sets.insert(nonterminal, set);
        }
    }

    fn build_sentential_tails(&mut self, nonterminal: &str) {
        for production in &self.grammar.production_map[nonterminal] {
            let tail_nonterminals = production
                .tokens
                .iter()
                .rev()
                .take_while(|t| self.grammar.nonterminals.contains(*t))
                .map(|t| t.as_str())
                .collect::<Vec<_>>();
            for last in tail_nonterminals {
                // examine nonterminals working backwards from the end of the
                // production. halt after encountering the first terminal
                let unseen = self.sets.get_mut(last).unwrap().insert(Vec::new());
                if unseen {
                    // avoid infinitely recursing in a production like `S: a S`
                    self.build_sentential_tails(last);
                }

                // if this nonterminal can expand to an empty string, then the
                // previous nonterminal in the production could also be a
                // sentential tail
                if !self.first.query_token(last).contains(&Vec::new()) {
                    break;
                }
            }
        }
    }
}
