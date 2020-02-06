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

use crate::grammar::Grammar;
use std::collections::{HashMap, HashSet};

fn is_epsilon(t: &str) -> bool {
    t == "\"\""
}

// fn vec_clone()
fn vec_set_crossproduct<'v, 'g: 'v>(
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
                    .filter(|t| !is_epsilon(*t))
                    .take(limit)
                    .cloned()
                    .collect()
            })
            .collect()
    } else {
        let mut output = HashSet::new();
        for left in lhs {
            if left.iter().filter(|t| !is_epsilon(*t)).count() >= limit {
                output.insert(left.clone());
                continue;
            }
            for right in rhs.clone() {
                output.insert(
                    left.iter()
                        .chain(right.iter())
                        .filter(|t| !is_epsilon(*t))
                        .take(limit)
                        .cloned()
                        .collect(),
                );
            }
        }
        output
    }
}

pub type FirstSet<'g> = HashSet<Vec<&'g str>>;

#[derive(Clone, Debug)]
pub struct First<'g> {
    k: usize,
    sets: HashMap<&'g str, FirstSet<'g>>,
}

impl<'g> First<'g> {
    pub fn new(grammar: &'g Grammar, k: usize) -> First<'g> {
        First {
            k,
            sets: FirstBuilder::new(grammar, k).build(),
        }
    }

    pub fn query_token(&self, nonterminal: &str) -> &FirstSet<'g> {
        &self.sets[nonterminal]
    }

    pub fn query_string(&self, string: &[&str]) -> FirstSet<'g> {
        let mut output = FirstSet::new();
        for token in string {
            output = vec_set_crossproduct(&output, self.query_token(token), self.k);
        }
        output
    }
}

struct FirstBuilder<'g> {
    grammar: &'g Grammar,
    k: usize,
    f: HashMap<&'g str, Vec<FirstSet<'g>>>,
}

impl<'g> FirstBuilder<'g> {
    fn new(grammar: &'g Grammar, k: usize) -> Self {
        Self {
            grammar,
            k,
            f: HashMap::new(),
        }
    }

    fn build(mut self) -> HashMap<&'g str, FirstSet<'g>> {
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
        let mut set_zero = FirstSet::new();
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

    fn get_f(&mut self, token: &'g str) -> &FirstSet<'g> {
        self.f[token].last().unwrap()
    }

    fn populate_nonterminal_i(&mut self, nonterminal: &'g str) -> bool {
        let k = self.k;
        let mut next = FirstSet::new();
        for prod in &self.grammar.production_map[nonterminal] {
            next.extend(prod.tokens.iter().fold(FirstSet::new(), |acc, x| {
                vec_set_crossproduct(&acc, self.get_f(x), k)
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
