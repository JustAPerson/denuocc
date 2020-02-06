# denuocc grammar_tool

This tool aims to help in the development of [predictive recursive descent parser][wiki].

To see the `FIRST` sets of every production in a grammar, use
```bash
cargo run -- first -d 2
```
[wiki]: https://en.wikipedia.org/wiki/Recursive_descent_parser

# Syntax

`grammar_tool` accepts a very simple grammar format similar to YACC. The input is
split into two parts: the header and the body, separated by a line of just `%%`.

The header may contain a `%token` or `%start` line. The `%token` line declares
identifiers that are terminals. The `%start` line specifies which nonterminal is
the root of the grammar.

The body contains the definitions of every nonterminal. A definition may provide
multiple productions using the `|` character. Any identifier referenced in a
production, if not declared by a `%token` header, is assumed to be a
nonterminal. A quoted string in a production is also a terminal.


```
%token TERMINAL
%start S
%%

S : variant1
  | variant2
  ;

variant1 : TERMINAL TERMINAL;
variant2 : "terminal" ;
```
