// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

use crate::front::location::Span;
use crate::front::token::syn_token::{Keyword, SynToken, SynTokenKind};
// use bitflags::bitflags;

// bitflags! {
//     struct StorageClass: u8 {
//         const TYPDEF = 0x01;
//         const EXTERN = 0x02;
//         const STATIC = 0x04;
//         const AUTO = 0x08;
//         const REGISTER = 0x10;
//     }
// }

// bitflags! {
//     struct TypeSpecifier: u16 {
//         const VOID = 0x0001;
//         const CHAR = 0x0002;
//         const SHORT = 0x0004;
//         const INT   = 0x0008;
//         const LONG  = 0x0010;
//         const FLOAT  = 0x0020;
//         const DOUBLE  = 0x0040;
//         const SIGNED  = 0x0080;
//         const UNSIGNED  = 0x0100;
//         const BOOL  = 0x0200;
//         const COMPLEX0 = 0x400;
//         const SUNION = 0x0800;
//         const ENUM = 0x1000;
//         const TYPEDEFNAME = 0x2000;
//     }
// }

// bitflags! {
// }

pub struct Expression;
pub enum StorageClassSpecifier {
    Typedef,
    Extern,
    Static,
    Auto,
    Register,
}

impl StorageClassSpecifier {
    pub fn from_keyword(token: SynToken, keyword: Keyword) -> Span<StorageClassSpecifier> {
        let specifier = match keyword {
            Keyword::Typedef => StorageClassSpecifier::Typedef,
            Keyword::Extern => StorageClassSpecifier::Extern,
            Keyword::Static => StorageClassSpecifier::Static,
            Keyword::Auto => StorageClassSpecifier::Auto,
            Keyword::Register => StorageClassSpecifier::Register,
            _ => unreachable!(),
        };
        token.span_value(specifier)
    }
}

// pub enum TypeSpecifierQualifier {
//     TypeSpecifier(TypeSpecifier),
//     TypeQualifier(TypeQualifier),
// }

// pub struct StructDeclarator {
// }
// pub struct StructDeclaration {
//     specquals: Vec<Span<TypeSpecifierQualifier>>,
//     declarators: Vec<Span<StructDeclarator>>,
// }

// pub struct StructUnionSpecifier {
//     name: Span<String>,
//     declarations: Vec<Span<StructDeclaration>>,
// }

// pub struct Enumerator {
//     name: Span<String>,
//     initializer: Span<Expression>,
// }

// pub struct EnumSpecifier {
//     name: Span<String>,
//     declarations: Vec<Span<Enumerator>>,
// }

pub enum TypeSpecifier {
    Void,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Signed,
    Unsigned,
    Bool,
    Complex,
    Imaginary,
    // Struct(StructUnionSpecifier),
    // Union(StructUnionSpecifier),
    // Enum(EnumSpecifier),
    TypedefName(String),
}

impl TypeSpecifier {
    pub fn from_keyword(token: SynToken, keyword: Keyword) -> Span<TypeSpecifier> {
        let specifier = match keyword {
            Keyword::Void => TypeSpecifier::Void,
            Keyword::Char => TypeSpecifier::Char,
            Keyword::Short => TypeSpecifier::Short,
            Keyword::Int => TypeSpecifier::Int,
            Keyword::Long => TypeSpecifier::Long,
            Keyword::Float => TypeSpecifier::Float,
            Keyword::Double => TypeSpecifier::Double,
            Keyword::Signed => TypeSpecifier::Signed,
            Keyword::Unsigned => TypeSpecifier::Unsigned,
            Keyword::Bool => TypeSpecifier::Bool,
            Keyword::Complex => TypeSpecifier::Complex,
            Keyword::Imaginary => TypeSpecifier::Imaginary,
            _ => unreachable!(),
        };
        token.span_value(specifier)
    }

    pub fn from_typedef(mut token: SynToken) -> Span<TypeSpecifier> {
        match &mut token.kind {
            SynTokenKind::String(s) => {
                let s = std::mem::take(s);
                token.span_value(TypeSpecifier::TypedefName(s))
            }
            _ => unreachable!(),
        }
    }
}

pub enum TypeQualifier {
    Const,
    Volatile,
    Restrict,
}

impl TypeQualifier {
    pub fn from_keyword(token: SynToken, keyword: Keyword) -> Span<TypeQualifier> {
        let qualifier = match keyword {
            Keyword::Const => TypeQualifier::Const,
            Keyword::Volatile => TypeQualifier::Volatile,
            Keyword::Restrict => TypeQualifier::Restrict,
            _ => unreachable!(),
        };
        token.span_value(qualifier)
    }
}

pub enum FunctionSpecifier {
    Inline,
}

pub enum DeclarationSpecifier {
    StorageClassSpecifier(StorageClassSpecifier),
    TypeSpecifier(TypeSpecifier),
    TypeQualifier(TypeQualifier),
    FunctionSpecifier(FunctionSpecifier),
}

impl From<StorageClassSpecifier> for DeclarationSpecifier {
    fn from(storage: StorageClassSpecifier) -> DeclarationSpecifier {
        DeclarationSpecifier::StorageClassSpecifier(storage)
    }
}

impl From<TypeSpecifier> for DeclarationSpecifier {
    fn from(specifier: TypeSpecifier) -> DeclarationSpecifier {
        DeclarationSpecifier::TypeSpecifier(specifier)
    }
}

impl From<TypeQualifier> for DeclarationSpecifier {
    fn from(qualifier: TypeQualifier) -> DeclarationSpecifier {
        DeclarationSpecifier::TypeQualifier(qualifier)
    }
}


impl From<FunctionSpecifier> for DeclarationSpecifier {
    fn from(specifier: FunctionSpecifier) -> DeclarationSpecifier {
        DeclarationSpecifier::FunctionSpecifier(specifier)
    }
}

pub type TypeQualifierList = Vec<TypeQualifier>;
pub struct Pointer(pub Vec<TypeQualifierList>);
pub struct Declarator;
pub struct Initializer;
pub struct InitDeclarator {
    declarator: Span<Declarator>,
    initializer: Span<Initializer>,
}
pub struct Declaration {
    specifiers: Vec<Span<DeclarationSpecifier>>,
    init_declarators: Vec<Span<InitDeclarator>>
}
pub struct FunctionDefinition;
pub enum ExternalDeclaration {
    Def(FunctionDefinition),
    Decl(Declaration),
}

pub struct TranslationUnit {
    pub declarations: Vec<Declaration>,
    pub definitions: Vec<FunctionDefinition>,
}
