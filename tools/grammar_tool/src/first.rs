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

use std::collections::{HashMap, HashSet};

use crate::grammar::Grammar;
use crate::token::{empty_string_set, string_set_crossproduct, StringSet};

#[derive(Clone, Debug)]
pub struct First<'g> {
    k: usize,
    sets: HashMap<&'g str, StringSet<'g>>,
}

impl<'g> First<'g> {
    pub fn new(grammar: &'g Grammar, k: usize) -> First<'g> {
        First {
            k,
            sets: FirstBuilder::new(grammar, k).build(),
        }
    }

    pub fn query_token(&self, nonterminal: impl AsRef<str>) -> &StringSet<'g> {
        &self.sets[nonterminal.as_ref()]
    }

    pub fn query_string(&self, string: impl IntoIterator<Item = impl AsRef<str>>) -> StringSet<'g> {
        string.into_iter().fold(empty_string_set(), |acc, x| {
            string_set_crossproduct(&acc, self.query_token(x), self.k)
        })
    }
}

struct FirstBuilder<'g> {
    grammar: &'g Grammar,
    k: usize,
    f: HashMap<&'g str, StringSet<'g>>,
}

impl<'g> FirstBuilder<'g> {
    fn new(grammar: &'g Grammar, k: usize) -> Self {
        Self {
            grammar,
            k,
            f: HashMap::new(),
        }
    }

    fn build(mut self) -> HashMap<&'g str, StringSet<'g>> {
        self.populate_terminals();
        self.populate_nonterminals();

        self.f
    }

    fn populate_terminals(&mut self) {
        for terminal in &self.grammar.terminals {
            let mut set = HashSet::new();
            set.insert(vec![terminal.as_str()]);

            self.f.insert(terminal, set);
        }
    }

    fn populate_nonterminals(&mut self) {
        for nonterminal in &self.grammar.nonterminals {
            self.populate_nonterminal_zero(nonterminal);
        }
        for _ in 1.. {
            let mut changes = Vec::new();
            for nonterminal in &self.grammar.nonterminals {
                if let Some(change) = self.populate_nonterminal_i(nonterminal) {
                    changes.push((nonterminal.as_str(), change));
                }
            }
            if changes.is_empty() {
                break;
            } else {
                self.f.extend(changes);
            }
        }
    }

    fn is_terminal(&self, token: &'g str) -> bool {
        self.grammar.terminals.contains(token)
    }

    fn populate_nonterminal_zero(&mut self, nonterminal: &'g str) {
        let mut set_zero = StringSet::new();
        for prod in &self.grammar.production_map[nonterminal] {
            let leading_terminals = prod
                .tokens
                .iter()
                .take_while(|t| self.is_terminal(t))
                .take(self.k)
                .count();
            if leading_terminals >= self.k || leading_terminals == prod.tokens.len() {
                set_zero.insert(
                    prod.tokens
                        .iter()
                        .take(leading_terminals)
                        .map(|t| t.as_str())
                        .collect(),
                );
            }
        }
        self.f.insert(nonterminal, set_zero);
    }

    fn populate_nonterminal_i(&mut self, nonterminal: &'g str) -> Option<StringSet<'g>> {
        let k = self.k;
        let mut next = StringSet::new();
        for prod in &self.grammar.production_map[nonterminal] {
            let prod_set = prod.tokens.iter().fold(empty_string_set(), |acc, x| {
                string_set_crossproduct(&acc, &self.f[x.as_str()], k)
            });
            next.extend(prod_set);
        }
        let prev = &self.f[nonterminal];
        if !next.is_subset(prev) {
            Some(&next | prev)
        } else {
            None
        }
    }
}
