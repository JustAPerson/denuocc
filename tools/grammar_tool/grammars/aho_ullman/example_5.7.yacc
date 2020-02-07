%token a b
%start S
%%

// Example 5.7 from Aho and Ullman, The Theory of Parsing, Translation, and Compiling.
// an LL(2) grammar

S : a S | a ;
