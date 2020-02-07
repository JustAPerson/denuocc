%token a b
%start S
%%

// Example 5.8 from Aho and Ullman, The Theory of Parsing, Translation, and Compiling.
// a weak LL(2) grammar

S : a A a a | b A b a ;
A : b | ;
