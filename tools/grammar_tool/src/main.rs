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

use std::str::FromStr;

#[macro_use]
extern crate lalrpop_util;

lalrpop_mod!(
    #[allow(unused_parens)]
    input_body
);

mod first;
mod follow;
mod grammar;
mod input_types;
mod token;

use first::First;
use follow::Follow;
use grammar::Grammar;

static AFTER_HELP: &str = "\
grammar_tool accepts a very simple grammar format similar to YACC. The input is
split into two parts: the header and the body, separated by a line of just `%%`.

The header may contain a `%token` or `%start` line. The `%token` line declares
identifiers that are terminals. The `%start` line specifies which nonterminal is
the root of the grammar.

The body contains the definitions of every nonterminal. A definition may provide
multiple productions using the `|` character. Any identifier referenced in a
production, if not declared by a `%token` header, is assumed to be a
nonterminal. A quoted string in a production is also a terminal.


    %token TERMINAL
    %start S
    %%

    S : variant1
      | variant2
      ;

    variant1 : TERMINAL TERMINAL;
    variant2 : \"terminal\" ;
";

fn generate_clap<'a, 'b>() -> clap::App<'a, 'b> {
    let file = clap::Arg::with_name("FILE").required(true);
    let k = clap::Arg::with_name("k")
        .help("Lookahead constant, or depth")
        .short("k")
        .default_value("1");

    clap::App::new("grammar_tool")
        .about("For manipulating grammars")
        .long_about(AFTER_HELP)
        .subcommand(
            clap::SubCommand::with_name("dot")
                .about("Create a dot(1) graph of the grammar")
                .arg(file.clone()),
        )
        .subcommand(
            clap::SubCommand::with_name("first")
                .about("Calculate the FIRST set of every production")
                .arg(file.clone())
                .arg(k.clone()),
        )
        .subcommand(
            clap::SubCommand::with_name("follow")
                .about("Calculate the FOLLOW set of every production")
                .arg(file.clone())
                .arg(k.clone()),
        )
        .subcommand(
            clap::SubCommand::with_name("print")
                .about("Print basic information about the grammar")
                .arg(file.clone()),
        )
        .subcommand(
            clap::SubCommand::with_name("test")
                .about("Test if grammar is LL(k) and if it is strong")
                .arg(file.clone())
                .arg(k.clone())
                .arg(
                    clap::Arg::with_name("explain")
                        .short("e")
                        .long("explain")
                        .help("Show details about conflicts"),
                ),
        )
}

fn main() {
    let clap = generate_clap().get_matches();

    match clap.subcommand() {
        ("dot", Some(matches)) => dot(matches),
        ("first", Some(matches)) => first(matches),
        ("follow", Some(matches)) => follow(matches),
        ("print", Some(matches)) => print(matches),
        ("test", Some(matches)) => test(matches),
        ("", _) => {
            generate_clap().print_help().unwrap();
            println!();
        }
        h => panic!("{:?}", h), // clap should've caught unknown subcommands
    }
}

fn get_grammar<'a>(flags: &clap::ArgMatches<'a>) -> Grammar {
    let input = std::fs::read_to_string(flags.value_of("FILE").unwrap()).unwrap();
    Grammar::from_str(&input).unwrap()
}

fn get_k<'a>(flags: &clap::ArgMatches<'a>) -> usize {
    flags
        .value_of("k")
        .map(|v| v.parse::<usize>().ok())
        .flatten()
        .expect("argument to -k must be a positive integer")
}

fn dot<'a>(flags: &clap::ArgMatches<'a>) {
    let grammar = get_grammar(flags);

    let mut references = Vec::<(&str, &str)>::new();
    for production in &grammar.productions {
        for token in &production.tokens {
            references.push((&production.name, &token));
        }
    }

    references.sort_unstable();
    references.dedup();

    println!("digraph {{");
    for (u, v) in references {
        println!("  {} -> {}", u, v);
    }
    println!("}}");
}

fn first<'a>(flags: &clap::ArgMatches<'a>) {
    let grammar = get_grammar(flags);
    let k = get_k(flags);
    let first = First::new(&grammar, k);

    for nonterminal in grammar.nonterminals_in_order() {
        let mut set = first
            .query_token(nonterminal)
            .iter()
            .collect::<Vec<&Vec<_>>>();
        set.sort();
        for seq in set {
            println!("{} : {}", nonterminal, seq.join(" "));
        }
    }
}

fn follow<'a>(flags: &clap::ArgMatches<'a>) {
    let grammar = get_grammar(flags);
    let k = get_k(flags);
    let first = First::new(&grammar, k);
    let follow = Follow::new(&grammar, &first);

    for nonterminal in grammar.nonterminals_in_order() {
        let mut set = follow
            .query_token(nonterminal)
            .iter()
            .collect::<Vec<&Vec<_>>>();
        set.sort();
        for seq in set {
            println!("{} : {}", nonterminal, seq.join(" "));
        }
    }
}

fn print<'a>(flags: &clap::ArgMatches<'a>) {
    let grammar = get_grammar(flags);
    let mut terminals = grammar.terminals.into_iter().collect::<Vec<_>>();
    terminals.sort();

    println!("start: {}", grammar.start);
    println!("terminals: {}", terminals.join(" "));
    for production in &grammar.productions {
        println!(
            "{} {} : {} ;",
            production.id,
            production.name,
            production.tokens.join(" ")
        );
    }
}

fn test<'a>(flags: &clap::ArgMatches<'a>) {
    let grammar = get_grammar(flags);
    let k = get_k(flags);
    let explain = flags.is_present("explain");
    let first = First::new(&grammar, k);
    let follow = Follow::new(&grammar, &first);

    let mut ll_k = true;
    let mut strong = true;
    for nonterminal in grammar.nonterminals_in_order() {
        let candidates = &grammar.production_map[nonterminal];
        if candidates.len() == 1 {
            // if this nonterminal has only one production, it cannot create an
            // LL(k) ambiguity
            continue;
        }

        let follows = follow.query_token(nonterminal);
        for i in 0..candidates.len() {
            for j in (i + 1)..candidates.len() {
                let mut all_first_a = token::StringSet::new();
                let mut all_first_b = token::StringSet::new();
                let mut sources = std::collections::HashMap::<Vec<&str>, Vec<usize>>::new();

                for f in follows {
                    let a = &candidates[i];
                    let b = &candidates[j];

                    let mut a_tokens = a.tokens.iter().map(|t| t.as_str()).collect::<Vec<_>>();
                    let mut b_tokens = b.tokens.iter().map(|t| t.as_str()).collect::<Vec<_>>();
                    a_tokens.extend(f);
                    b_tokens.extend(f);

                    let first_a = first.query_string(a_tokens);
                    let first_b = first.query_string(b_tokens);

                    for fa in &first_a {
                        sources.entry(fa.clone()).or_insert(Vec::new()).push(a.id);
                        all_first_a.insert(fa.clone());
                    }
                    for fb in &first_b {
                        sources.entry(fb.clone()).or_insert(Vec::new()).push(b.id);
                        all_first_b.insert(fb.clone());
                    }

                    let conflicts = first_a.intersection(&first_b).collect::<Vec<_>>();
                    if !conflicts.is_empty() {
                        ll_k = false;
                        if explain {
                            println!(
                                "productions {:?} cause LL-conflicts: {:?}",
                                [a.id, b.id],
                                conflicts
                            );
                            println!(
                                "  production {}   {} : {};",
                                a.id,
                                &a.name,
                                a.tokens.join(" ")
                            );
                            println!(
                                "  production {}   {} : {};",
                                b.id,
                                &b.name,
                                b.tokens.join(" ")
                            );
                            println!("  conflicting suffix: {:?}", &f);
                        }
                    }
                }
                // strong conflicts
                if ll_k {
                    // only an LL(k) grammar can be strong
                    for conflict in all_first_a.intersection(&all_first_b) {
                        debug_assert!(sources[conflict].len() >= 2);
                        strong = false;

                        if explain {
                            println!(
                                "productions {:?} cause strong-LL-conflict: {:?}",
                                &sources[conflict], conflict
                            )
                        }
                    }
                }
            }
        }
    }

    match (ll_k, strong) {
        (true, true) => println!("grammar is strong LL({})", k),
        (true, false) => println!("grammar is weak LL({})", k),
        (false, _) => println!("grammar is not LL({})", k),
    }
}
