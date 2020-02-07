%token a b
%start S
%%

// Example 5.2 from Aho and Ullman, The Theory of Parsing, Translation, and Compiling.
// an LL(1) grammar

S : a A S | b ;
A : a | b S A ;
