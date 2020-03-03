%token identifier constant string_literal typedef_name
%token storage_class_specifier type_specifier_lit type_qualifier struct_or_union
%%

primary_expression
  : identifier
  | constant
  | string_literal
  | "(" expression ")"
  ;

postfix_expression
  : postfix_expression_begin postfix_expression_tail
  ;

postfix_expression_begin
  : primary_expression
  | "(" type_name ")" "{" initializer_list optional_comma_bracket
  ;

optional_comma_bracket
  : "}"
  | "," "}"
  ;

postfix_expression_tail
  :
  | "[" expression "]" postfix_expression_tail
  | "(" argument_expression_list ")" postfix_expression_tail
  | "(" ")" postfix_expression_tail
  | "." identifier postfix_expression_tail
  | "->" identifier postfix_expression_tail
  | "++" postfix_expression_tail
  | "--" postfix_expression_tail
  ;


argument_expression_list
  : assignment_expression argument_expression_list_prime
  ;

argument_expression_list_prime
  :
  | "," assignment_expression argument_expression_list_prime
  ;

unary_expression
  : postfix_expression
  | unary_common
  ;

unary_common
  : "++" unary_expression
  | "--" unary_expression
  | unary_operator cast_expression
  | "sizeof" hacked_unary_expression
  ;

// exists because sizeof can be followed by "(" type-name ")" OR a
// unary_expression (and indirectly a postfix_expression) and guess what can
// start a postfix_expression...
hacked_unary_expression
  : hacked_postfix_expression
  | unary_common
  ;

hacked_postfix_expression
  : hacked_postfix_expression_begin postfix_expression_tail
  ;

hacked_postfix_expression_begin
  : primary_expression
  | "(" type_name ")" hacked_postfix_expression_initializer
  ;

hacked_postfix_expression_initializer
  :
  | "{"  initializer_list optional_comma_bracket
  ;

unary_operator
  : "&"
  | "*"
  | "+"
  | "-"
  | "~"
  | "!"
  ;

// cast_expression
//   : unary_expression
//   | "(" type_name ")" cast_expression
//   ;

cast_expression
  : hacked_unary_expression
  ;

multiplicative_expression
  : cast_expression multiplicative_expression_prime
  ;

multiplicative_expression_prime
  :
  | "*" cast_expression multiplicative_expression_prime
  | "/" cast_expression multiplicative_expression_prime
  | "%" cast_expression multiplicative_expression_prime
  ;

additive_expression
  : multiplicative_expression additive_expression_prime
  ;

additive_expression_prime
  :
  | "+" multiplicative_expression additive_expression_prime
  | "-" multiplicative_expression additive_expression_prime
  ;

shift_expression
  : additive_expression shift_expression_prime
  ;

shift_expression_prime
  :
  | "<<" additive_expression shift_expression_prime
  | ">>" additive_expression shift_expression_prime
  ;

relational_expression
  : shift_expression relational_expression_prime
  ;

relational_expression_prime
  :
  | "<" shift_expression relational_expression_prime
  | ">" shift_expression relational_expression_prime
  | "<=" shift_expression relational_expression_prime
  | ">=" shift_expression relational_expression_prime
  ;

equality_expression
  : relational_expression equality_expression_prime
  ;

equality_expression_prime
  :
  | "==" relational_expression equality_expression_prime
  | "!=" relational_expression equality_expression_prime
  ;

and_expression
  : equality_expression and_expression_prime
  ;

and_expression_prime
  :
  | "&" equality_expression and_expression_prime
  ;

exclusive_or_expression
  : and_expression logical_or_expression_prime
  ;

exclusive_or_expression_prime
  :
  | "^" and_expression logical_or_expression_prime
  ;

inclusive_or_expression
  : exclusive_or_expression inclusive_or_expression_prime
  ;

inclusive_or_expression_prime
  :
  | "|" exclusive_or_expression inclusive_or_expression_prime
  ;

logical_and_expression
  : inclusive_or_expression logical_and_expression_prime
  ;

logical_and_expression_prime
  :
  | "&&" inclusive_or_expression logical_and_expression_prime
  ;

logical_or_expression
  : logical_and_expression logical_or_expression_prime
  ;

logical_or_expression_prime
  :
  | "||" logical_and_expression logical_or_expression_prime
  ;

conditional_expression
  : logical_or_expression conditional_expression_prime
  ;

conditional_expression_prime
  :
  | "?" expression ":" conditional_expression
  ;

// hack: allow lhs of assignment to be a binary expression and diagnose later
assignment_expression
  : conditional_expression assignment_expression_end
  ;

assignment_expression_end
  :
  | assignment_operator assignment_expression
  ;

assignment_operator
  : "="
  | "*="
  | "/="
  | "%="
  | "+="
  | "-="
  | "<<="
  | ">>="
  | "&="
  | "^="
  | "|="
  ;

expression
  : assignment_expression expression_prime
  ;

expression_prime
  :
  | "," assignment_expression expression_prime
  ;

expression_opt
  :
  | expression
  ;

constant_expression
  : conditional_expression
  ;

declaration
  : declaration_specifiers declaration_end
  ;

declaration_end
  : ";"
  | init_declarator_list ";"
  ;


// declaration_specifiers
//   : storage_class_specifier
//   | storage_class_specifier declaration_specifiers
//   | type_specifier
//   | type_specifier declaration_specifiers
//   | type_qualifier
//   | type_qualifier declaration_specifiers
//   | function_specifier
//   | function_specifier declaration_specifiers
//   ;

declaration_specifiers : declaration_specifiers_lf declaration_specifiers_prime;

declaration_specifiers_lf
  : storage_class_specifier
  | type_specifier
  | type_qualifier
  | function_specifier
  ;

declaration_specifiers_prime
  :
  | declaration_specifiers_lf declaration_specifiers_prime
  ;

init_declarator_list
  : init_declarator init_declarator_list_prime
  ;

init_declarator_list_prime
  :
  | "," init_declarator init_declarator_list_prime
  ;

init_declarator : declarator init_declarator_lf ;

init_declarator_lf
  :
  | "=" initializer
  ;

// storage_class_specifier
//   : "typedef"
//   | "extern"
//   | "static"
//   | "auto"
//   | "register"
//   ;

type_specifier
  : type_specifier_lit
  | struct_or_union_specifier
  | enum_specifier
  | typedef_name
  ;

struct_or_union_specifier
  : struct_or_union struct_or_union_specifier_end
  ;

// struct_or_union : "struct" | "union" ;

struct_or_union_specifier_end
  : "{" struct_declaration_list "}"
  | identifier "{" struct_declaration_list "}"
  | identifier
  ;


struct_declaration_list
  : struct_declaration struct_declaration_list_prime
  ;

struct_declaration_list_prime
  :
  | struct_declaration struct_declaration_list_prime
  ;

struct_declaration
  : specifier_qualifier_list struct_declarator_list ";"
  ;

specifier_qualifier_list : specifier_qualifier_lf specifier_qualifier_list_prime ;

specifier_qualifier_lf
  : type_specifier
  | type_qualifier
  ;

specifier_qualifier_list_prime
  :
  | specifier_qualifier_lf specifier_qualifier_list_prime
  ;

struct_declarator_list
  : struct_declarator struct_declarator_list_prime
  ;

struct_declarator_list_prime
  :
  | "," struct_declarator struct_declarator_list_prime
  ;

struct_declarator
  : declarator struct_declarator_prime
  | ":" constant_expression
  ;

struct_declarator_prime
  :
  | ":" constant_expression
  ;


enum_specifier
  : "enum" enum_specifier_mid
  ;

enum_specifier_mid
  : "{" enumerator_list optional_comma_bracket
  | identifier "{" enumerator_list optional_comma_bracket
  | identifier
  ;

enumerator_list
  : enumerator enumerator_list_prime
  ;

enumerator_list_prime
  :
  | "," enumerator enumerator_list_prime
  ;

enumerator
  : identifier
  | identifier "=" constant_expression
  ;

// type_qualifier
//   : "const"
//   | "restrict"
//   | "volatile"
//   ;

function_specifier
  : "inline"
  ;

declarator
  : direct_declarator
  | pointer direct_declarator
  ;

direct_declarator
  : direct_declarator_first direct_declarator_recursive
  ;

direct_declarator_first
  : identifier
  | "(" declarator ")"
  ;

direct_declarator_recursive
  : "[" direct_declarator_recursive_in_brackets "]"
  | "(" parameter_type_list ")"
  | "(" identifier_list ")"
  | "(" ")"
  ;

direct_declarator_recursive_in_brackets
  :
  | assignment_expression
  | "static" assignment_expression
  | "static" type_qualifier_list assignment_expression
  | type_qualifier_list direct_declarator_recursive_tq
  | "*"
  ;

direct_declarator_recursive_tq
  :
  | assignment_expression
  | "static" assignment_expression
  | "*"
  ;


pointer
  : "*" type_qualifier_list_opt pointer_prime
  ;

pointer_prime
  :
  | pointer
  ;

type_qualifier_list
  : type_qualifier type_qualifier_list_opt
  ;

type_qualifier_list_opt
  :
  | type_qualifier type_qualifier_list
  ;

parameter_type_list
  : parameter_list parameter_type_list_end
  ;

parameter_type_list_end
  :
  | "," "..."
  ;

parameter_list
  : parameter_declaration parameter_list_prime
  ;

parameter_list_prime
  : "," parameter_declaration parameter_list_prime
  ;

parameter_declaration
  : declaration_specifiers parameter_declaration_end
  ;

parameter_declaration_end
  :
  | combined_declarator
  ;

combined_declarator
  : pointer direct_combined_declarator_opt
  | direct_combined_declarator
  ;

direct_combined_declarator_opt
  :
  | direct_combined_declarator
  ;

// pretty sure this is wrong, but it can be handled in parser or postprocessing.
direct_combined_declarator
  : identifier direct_declarator_recursive
  | direct_declarator_recursive
  ;

identifier_list
  : identifier identifier_list_prime
  ;

identifier_list_prime
  : "," identifier identifier_list_prime
  ;

type_name
  : specifier_qualifier_list type_name_end
  ;

type_name_end
  :
  | abstract_declarator
  ;


abstract_declarator
  : direct_abstract_declarator
  | pointer direct_abstract_declarator_opt
  ;

direct_abstract_declarator
  : direct_abstract_declarator_first direct_declarator_recursive
  ;

direct_abstract_declarator_opt
  :
  | direct_abstract_declarator
  ;

direct_abstract_declarator_first
  :
  | "(" abstract_declarator ")"
  ;

direct_abstract_declarator_recursive
  : "[" direct_abstract_declarator_recursive_in_brackets "]"
  | "(" parameter_type_list ")"
  | "(" identifier_list ")"
  | "(" ")"
  ;

direct_abstract_declarator_recursive_in_brackets
  :
  | assignment_expression
  | "static" assignment_expression
  | "static" type_qualifier_list assignment_expression
  | type_qualifier_list direct_abstract_declarator_recursive_tq
  | "*"
  ;

direct_abstract_declarator_recursive_tq
  :
  | assignment_expression
  | "static" assignment_expression
  | "*"
  ;

assignment_expression_opt
  :
  | assignment_expression
  ;

initializer
  : assignment_expression
  | "{" initializer_list initializer_end
  ;
initializer_end
  : "}"
  | "," "}"
  ;

initializer_list
  : designation_opt initializer initializer_list_prime
  ;

initializer_list_prime
  :
  | "," designation_opt initializer initializer_list_prime
  ;

designation_opt
  :
  | designator_list "="
  ;

designator_list
  : designator designator_list_prime
  ;

designator_list_prime
  :
  | designator designator_list_prime
  ;

designator
  : "[" constant_expression "]"
  | "." identifier
  ;

statement
  : labeled_statement
  | compound_statement
  | expression_statement
  | selection_statement
  | iteration_statement
  | jump_statement
  ;

labeled_statement
  : identifier ":" statement
  | "case" constant_expression ":" statement
  | "default" ":" statement
  ;


compound_statement
  : "{" block_item_list_opt "}"
  ;

block_item_list
  : block_item block_item_list_opt
  ;

block_item_list_opt
  :
  | block_item block_item_list_opt
  ;

block_item
  : declaration
  | statement
  ;

expression_statement
  : expression_opt ";"
  ;

selection_statement
  : "if" "(" expression ")" statement if_statement_end
  | "switch" "(" expression ")" statement
  ;

if_statement_end
  :
  | "else" statement
  ;

iteration_statement
  : "while" "(" expression ")" statement
  | "do" statement "while" "(" expression ")" ";"
  | "for" "(" for_condition ")" statement
  ;

for_condition
  : expression_opt ";" expression_opt ";" expression_opt
  | declaration expression_opt ";" expression_opt
  ;

jump_statement
  : "goto" identifier ";"
  | "continue" ";"
  | "break" ";"
  | "return" expression_opt ";"
  ;

translation_unit
  : external_declaration translation_unit_prime
  ;

translation_unit_prime
  :
  | external_declaration translation_unit_prime
  ;

external_declaration
  : declaration_specifiers external_declaration_end
  ;

external_declaration_end
  : declaration_end
  | function_definition_remaining
  ;

function_definition_remaining
  : declaration_specifiers declarator declaration_list_opt compound_statement
  ;

declaration_list_opt
  :
  | declaration declaration_list_opt
  ;
