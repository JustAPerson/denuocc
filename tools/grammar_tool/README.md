# denuocc grammar_tool

This tool aims to help in the development of [predictive recursive descent parser][wiki].

[wiki]: https://en.wikipedia.org/wiki/Recursive_descent_parser

There are four main subcommand for `grammar_tool`:

- `print` shows and numbers every production in the grammar
  ```bash
  $ cargo run -- print ./grammars/aho_ullman/example_5.3.yacc
  start: S
  terminals: a b
  0 S :  ;
  1 S : a b A ;
  2 A : S a a ;
  3 A : b ;
  ```
  It shows the `start` nonterminal, which happens to be `S`. It shows that the
  terminals in this grammar are `a` and `b`. It then shows the 4 productions in
  this grammar.

- `first` shows the first set for every nonterminal in the grammar
  ```bash
  $ cargo run -- first -k2 ./grammars/aho_ullman/example_5.3.yacc 
  S : 
  S : a b
  A : a a
  A : a b
  A : b
  ```
  This means that the production `S` can legally start with an empty string or the
  token string `"ab"`. Similarly, the nonterminal `A` may begin with the token
  strings `"aa"`, `"ab"`, or just `"b"`. Notably, `A` cannot be an empty string.

- `follow` shows the follow set for every nonterminal in a grammar
  ```bash
  $ cargo run -- follow -k2 ./grammars/aho_ullman/example_5.3.yacc 
  S :
  S : a a
  A : 
  ```
  The `S : ` line with nothing after the colon means that is valid for the input
  string to end after parsing an `S`. The same goes for the line `A : `. The
  line `S : a a` means that the token string `"aa"` may sometimes legally
  follow after parsing an `S`.

- `test` will verify if a grammar if `LL(k)` or explain why it is not
  ```bash
  $ cargo run -- test -k1 ./grammars/aho_ullman/example_5.3.yacc 
  grammar is not LL(1)
  $ cargo run -- test -k1 --explain ./grammars/aho_ullman/example_5.3.yacc 
  productions [0, 1] cause LL-conflicts: [["a"]]
    production 0   S : ;
    production 1   S : a b A;
    conflicting suffix: ["a"]
  grammar is not LL(1)
  $ cargo run -- test -k2 ./grammars/aho_ullman/example_5.3.yacc 
  grammar is strong LL(2)
  ```
  As the `follow` command will show, an `S` production may be followed by the
  sequence `a a` (which occurs in production #2 `A : S a a`). Thus, it is
  impossible to tell with only 1 token lookahead which of the two `S`
  productions to choose.

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
