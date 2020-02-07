%token a b
%start S
%%

// Example 5.10 from Aho and Ullman, The Theory of Parsing, Translation, and Compiling.
// a strong LL(1) grammar

S : b Sprime;
Sprime : a Sprime | ;
