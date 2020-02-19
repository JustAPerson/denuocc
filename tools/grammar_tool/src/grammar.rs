// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

use std::collections::{HashMap, HashSet};

use crate::input_types::*;

#[derive(Clone, Debug)]
pub struct Production {
    pub name: String,
    pub id: usize,
    pub tokens: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Grammar {
    pub start: String,
    pub declared_terminals: HashSet<String>,
    pub terminals: HashSet<String>,
    pub nonterminals: HashSet<String>,
    pub productions: Vec<Production>,
    pub production_map: HashMap<String, Vec<Production>>,
}

fn assert_identifier(input: &str, line_num: usize) {
    let mut chars = input.chars();
    let first = chars
        .next()
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(true);
    let rest = chars.all(|c| c.is_ascii_alphanumeric() || c == '_');

    if !(first && rest) {
        panic!("{}: invalid identifier `{}`", line_num, input)
    }
}

impl std::str::FromStr for Grammar {
    type Err = ();
    fn from_str(input: &str) -> Result<Self, ()> {
        let (mut start, declared_terminals, body) = parse_header(input);
        let definitions = crate::input_body::BodyParser::new().parse(body).unwrap();

        let terms = definitions
            .iter()
            .map(|d| d.alternates.iter())
            .flatten()
            .flatten();

        let nonterminals = definitions
            .iter()
            .map(|d| d.name.clone())
            .collect::<HashSet<_>>();
        let string_terminals = terms
            .clone()
            .map(|term| {
                if let Term::String(s) = term {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect::<HashSet<_>>();

        let valid_identifiers = &declared_terminals | &nonterminals;
        terms.clone().for_each(|term| {
            if let Term::Identifier(s) = term {
                if !valid_identifiers.contains(s) {
                    panic!("missing nonterminal `{}`", s);
                }
            }
        });

        start = start.or_else(|| definitions.first().map(|d| d.name.clone()));

        let mut productions = Vec::new();
        for definition in definitions {
            for alternate in definition.alternates {
                productions.push(Production {
                    name: definition.name.clone(),
                    id: productions.len(),
                    tokens: alternate.iter().map(|t| t.as_ref().to_owned()).collect(),
                });
            }
        }
        productions.sort_by(|a, b| (a.id, &a.name).cmp(&(b.id, &b.name)));

        let mut production_map = HashMap::new();
        for production in &productions {
            production_map
                .entry(production.name.clone())
                .or_insert(Vec::new())
                .push(production.clone());
        }

        Ok(Grammar {
            start: start.unwrap(),
            terminals: &string_terminals | &declared_terminals,
            declared_terminals,
            nonterminals,
            productions,
            production_map,
        })
    }
}

fn parse_header(input: &str) -> (Option<String>, HashSet<String>, &str) {
    let mut declared_terminals: HashSet<String> = HashSet::new();
    let mut declared_start: Option<String> = None;
    let mut line_num: usize = 0;

    if !input.lines().any(|line| line == "%%") {
        return (None, HashSet::new(), input);
    }

    let mut remaining = input;
    for line in input.lines() {
        line_num += 1;
        remaining = &remaining[line.as_bytes().len() + 1..];

        if line.starts_with("%token ") {
            declared_terminals.extend(
                line.split_ascii_whitespace()
                    .skip(1)
                    .inspect(|w| assert_identifier(w, line_num))
                    .map(|w| w.to_owned()),
            );
        } else if line.starts_with("%start ") {
            let mut iter = line
                .split_ascii_whitespace()
                .skip(1)
                .inspect(|w| assert_identifier(w, line_num))
                .map(|w| w.to_owned());
            declared_start = iter.next();
            if declared_start.is_none() {
                panic!("{}: expected identifier after `%start`", line_num);
            }
            if iter.next().is_some() {
                panic!("{}: expected newline after identifier", line_num);
            }
        } else if line == "%%" {
            break;
        }
    }

    (declared_start, declared_terminals, remaining)
}

impl Grammar {
    pub fn nonterminals_in_order(&self) -> impl Iterator<Item = &str> {
        let mut pairs = self
            .nonterminals
            .iter()
            .map(|n| (self.production_map[n][0].id, n.as_str()))
            .collect::<Vec<_>>();
        pairs.sort();
        pairs.into_iter().map(|(_, p)| p)
    }
}
