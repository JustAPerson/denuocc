// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Phase 7: Parsing

use crate::front::location::Span;
use crate::front::message::MessageKind;
use crate::front::token::syn_token::{FastKeyword, Keyword, Punctuator};
use crate::front::token::{PPToken, PPTokenKind};
use crate::front::token::{SynToken, SynTokenKind};
use crate::tu::TUCtx;

pub mod syntax;
pub mod scopes;

pub use scopes::Scopes;

type Tokens = std::collections::VecDeque<SynToken>;

fn chop_single_quotes(mut input: String) -> String {
    debug_assert!(!input.is_empty());
    debug_assert_eq!(&input[0..1], "\'");
    debug_assert_eq!(&input[input.len() - 1..], "\'");
    input.pop(); // remove closing single quote
    input.remove(0); // remove opening single quote
    input
}

fn chop_double_quotes(mut input: String) -> String {
    debug_assert!(!input.is_empty());
    debug_assert_eq!(&input[0..1], "\"");
    debug_assert_eq!(&input[input.len() - 1..], "\"");
    input.pop(); // remove closing double quote
    input.remove(0); // remove opening double quote
    input
}

fn categorize_identifier(value: String) -> SynTokenKind {
    if let Some(fk) = FastKeyword::from_str(&value) {
        SynTokenKind::Keyword(fk)
    } else {
        SynTokenKind::Identifier(value)
    }
}

fn prepare_tokens(tuctx: &mut TUCtx, tokens: Vec<PPToken>) -> Tokens {
    tokens
        .into_iter()
        .flat_map(move |token| {
            let kind = match token.kind {
                // PPTokenKind::Identifier => SynTokenKind::Identifier(t.value),
                // PPTokenKind::IdentifierNonExpandable =>  SynTokenKind::Identifier(t.value),
                PPTokenKind::Other => {
                    tuctx.emit_message(
                        token.location,
                        MessageKind::Phase7UnrecognizedCharacter {
                            character: token.value.chars().next().unwrap(),
                        },
                    );
                    return None;
                },
                PPTokenKind::EndOfFile | PPTokenKind::Whitespace => return None,
                PPTokenKind::Identifier | PPTokenKind::IdentifierNonExpandable => {
                    categorize_identifier(token.value)
                },
                PPTokenKind::PPNumber => SynTokenKind::Number(token.value),
                PPTokenKind::StringLiteral => SynTokenKind::String(chop_double_quotes(token.value)),
                PPTokenKind::CharacterConstant => {
                    SynTokenKind::Character(chop_single_quotes(token.value))
                },
                PPTokenKind::Punctuator => {
                    SynTokenKind::Punctuator(Punctuator::from_str(&token.value).unwrap())
                },
            };
            Some(SynToken {
                location: token.location,
                kind,
            })
        })
        .collect()
}

fn starts_storage_class_specifier(token: &SynToken) -> bool {
    match token.kind {
        SynTokenKind::Keyword(fk) => fk.is_storage_class_specifier(),
        _ => false,
    }
}

fn starts_type_specifier(scopes: &mut Scopes, token: &SynToken) -> bool {
    match &token.kind {
        SynTokenKind::Keyword(fk) => fk.is_type_specifier(),
        SynTokenKind::Identifier(s) => scopes.contains_typedef(s.as_str()),
        _ => false,
    }
}

fn starts_type_qualifier(token: &SynToken) -> bool {
    match token.kind {
        SynTokenKind::Keyword(fk) => fk.is_type_qualifier(),
        _ => false,
    }
}

fn starts_function_specifier(token: &SynToken) -> bool {
    match token.kind {
        SynTokenKind::Keyword(fk) => fk.value() == Keyword::Inline,
        _ => false,
    }
}

fn parse_type_specifier(
    _tuctx: &mut TUCtx,
    _scopes: &mut Scopes,
    tokens: &mut Tokens,
) -> Option<Span<syntax::TypeSpecifier>> {
    let token = tokens.pop_front().unwrap();
    let spec = match &token.kind {
        SynTokenKind::Keyword(fk) if fk.is_simple_type_specifier() => {
            let keyword = fk.value();
            syntax::TypeSpecifier::from_keyword(token, keyword)
        },
        SynTokenKind::Keyword(fk) if fk.is_type_specifier() => {
            unimplemented!("TODO parse struct/union/enum specifiers");
        },
        SynTokenKind::Identifier(_) => {
            syntax::TypeSpecifier::from_typedef(token)
        },
        _ => unreachable!(),
    };

    Some(spec)
}

fn parse_declaration_specifiers(
    tuctx: &mut TUCtx,
    scopes: &mut Scopes,
    tokens: &mut Tokens,
) -> Vec<Span<syntax::DeclarationSpecifier>> {
    use syntax::{StorageClassSpecifier, TypeQualifier};
    let mut specifiers = Vec::<Span<syntax::DeclarationSpecifier>>::new();

    while let Some(token) = tokens.front() {
        if starts_storage_class_specifier(&token) {
            let token = tokens.pop_front().unwrap();
            let keyword = token.as_keyword().unwrap().value();
            specifiers.push(StorageClassSpecifier::from_keyword(token, keyword).into());
        } else if starts_type_specifier(scopes, &token) {
            if let Some(specifier) = parse_type_specifier(tuctx, scopes, tokens) {
                specifiers.push(specifier.into());
            }
        } else if starts_type_qualifier(&token) {
            let token = tokens.pop_front().unwrap();
            let keyword = token.as_keyword().unwrap().value();
            specifiers.push(TypeQualifier::from_keyword(token, keyword).into());
        } else if starts_function_specifier(&token) {
            let token = tokens.pop_front().unwrap();
            specifiers.push(token.span_value(syntax::FunctionSpecifier::Inline).into());
        } else {
            break;
        }
    }

    specifiers
}

fn parse_pointer(_tuctx: &mut TUCtx, _scopes: &mut Scopes, tokens: &mut Tokens) -> Option<syntax::Pointer> {
    let is_star = |t: &SynToken| { t.is_punctuator(Punctuator::Star) };

    if tokens.front().map(is_star) != Some(true) {
        return None
    }

    let mut pointers = Vec::new();
    while let Some(_) = tokens.front().filter(|t| is_star(t)) {
        let token = tokens.pop_front();
        let quals = Vec::new();
        pointers.push(quals);
    }
    Some(syntax::Pointer(pointers))
}

fn parse_declarator(_tuctx: &mut TUCtx, _scopes: &mut Scopes, tokens: &mut Tokens) -> Option<Span<syntax::Declarator>> {
    None
}

/// Parse either a _declaration_ or a _function-definition_
///
/// Both of these structures begin with a _declaration-specifier_ followed by a
/// _declarator_. After the first _declarator_, we can disambiguate which
/// structure to parse. A comma, equal, or semicolon indicate a _declaration_.
fn parse_external_declaration(
    tuctx: &mut TUCtx,
    scopes: &mut Scopes,
    tokens: &mut Tokens,
) -> Option<syntax::ExternalDeclaration> {
    let declspecs = parse_declaration_specifiers(tuctx, scopes, tokens);
    let declaration = parse_declarator(tuctx, scopes, tokens)?;
    None
}


pub fn parse(tuctx: &mut TUCtx, tokens: Vec<PPToken>) -> syntax::TranslationUnit {
    let mut tokens = prepare_tokens(tuctx, tokens);
    let mut tu = syntax::TranslationUnit {
        declarations: Vec::new(),
        definitions: Vec::new(),
    };

    let mut scopes = Scopes::new();
    scopes.start_scope();

    while let Some(ext_decl) = parse_external_declaration(tuctx, &mut scopes, &mut tokens) {
        match ext_decl {
            syntax::ExternalDeclaration::Def(def) => tu.definitions.push(def),
            syntax::ExternalDeclaration::Decl(decl) => tu.declarations.push(decl),
        }
    }
    // acknowledge and consume erroneous spare tokens in parse_external_declaration()
    assert!(tokens.is_empty());

    tu
}
