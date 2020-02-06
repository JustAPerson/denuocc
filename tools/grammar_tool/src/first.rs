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
use crate::token::{string_set_crossproduct, StringSet};

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
        let mut output = StringSet::new();
        for token in string {
            output = string_set_crossproduct(&output, self.query_token(token), self.k);
        }
        output
    }
}

struct FirstBuilder<'g> {
    grammar: &'g Grammar,
    k: usize,
    f: HashMap<&'g str, Vec<StringSet<'g>>>,
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
            .into_iter()
            .map(|(k, mut v)| (k, v.pop().unwrap()))
            .collect()
    }
    fn populate_terminals(&mut self) {
        for terminal in &self.grammar.terminals {
            let mut set = HashSet::new();
            set.insert(vec![terminal.as_str()]);

            self.f.insert(terminal, vec![set]);
        }
    }

    fn populate_nonterminals(&mut self) {
        for nonterminal in &self.grammar.nonterminals {
            self.populate_nonterminal_zero(nonterminal);
        }
        for _ in 1.. {
            let mut done = true;
            for nonterminal in &self.grammar.nonterminals {
                done &= self.populate_nonterminal_i(nonterminal);
            }
            if done {
                break;
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
        self.f.insert(nonterminal, vec![set_zero]);
    }

    fn get_f(&mut self, token: &'g str) -> &StringSet<'g> {
        self.f[token].last().unwrap()
    }

    fn populate_nonterminal_i(&mut self, nonterminal: &'g str) -> bool {
        let k = self.k;
        let mut next = StringSet::new();
        for prod in &self.grammar.production_map[nonterminal] {
            next.extend(prod.tokens.iter().fold(StringSet::new(), |acc, x| {
                string_set_crossproduct(&acc, self.get_f(x), k)
            }));
        }
        let prev = self.get_f(nonterminal);
        if !next.is_subset(prev) {
            let next = &next | prev;
            self.f.get_mut(nonterminal).unwrap().push(next);
            false
        } else {
            true
        }
    }
}
