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
mod grammar;
mod input_types;

use first::First;
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
    clap::App::new("grammar_tool")
        .about("For manipulating grammars")
        .long_about(AFTER_HELP)
        .subcommand(
            clap::SubCommand::with_name("dot")
                .about("Create a dot(1) graph of the grammar")
                .arg(file.clone()),
        )
        .subcommand(
            clap::SubCommand::with_name("print")
                .about("Print basic information about the grammar")
                .arg(file.clone()),
        )
        .subcommand(
            clap::SubCommand::with_name("first")
                .about("Calculate the FIRST set of every production")
                .arg(file.clone())
                .arg(
                    clap::Arg::with_name("k")
                        .help("Lookahead constant, or depth")
                        .short("k")
                        .default_value("1"),
                ),
        )
}

fn main() {
    let clap = generate_clap().get_matches();

    match clap.subcommand() {
        ("dot", Some(matches)) => dot(matches),
        ("first", Some(matches)) => first(matches),
        ("print", Some(matches)) => print(matches),
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
    let k = flags
        .value_of("k")
        .map(|v| v.parse::<usize>().ok())
        .flatten()
        .expect("argument to -k must be a positive integer");
    let first = First::new(&grammar, k);

    let mut prods = grammar
        .nonterminals
        .iter()
        .map(|n| (grammar.production_map[n][0].id, n))
        .collect::<Vec<_>>();
    prods.sort();
    for (_, nonterminal) in prods {
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

fn print<'a>(flags: &clap::ArgMatches<'a>) {
    let grammar = get_grammar(flags);
    let mut terminals = grammar.terminals.into_iter().collect::<Vec<_>>();
    terminals.sort();

    println!("start: {}", grammar.start);
    println!("terminals: {}", terminals.join(" "));
    for production in &grammar.productions {
        println!("{} : {} ;", production.name, production.tokens.join(" "));
    }
}
