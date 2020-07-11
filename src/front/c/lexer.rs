// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Phase 3: Construct preprocessor tokens

use std::rc::Rc;

use lazy_static::lazy_static;
use log::{debug, log_enabled, trace};
use regex::{Regex, RegexSet};

use super::token::TokenOrigin;
use crate::front::c::input::Input;
use crate::front::c::message::MessageKind;
use crate::front::c::token::{CharToken, PPToken, PPTokenKind, TextPosition, TextSpan};
use crate::front::c::tuctx::TUCtx;

static TOKEN_PATTERNS: &[(&'static str, PPTokenKind)] = &[
    ("^.", PPTokenKind::Other),
    (r"^(( |\f|\r|\t|\v)+|(\n))", PPTokenKind::Whitespace),
    (r"^((//.+)|(?s:/\*.*?\*/))", PPTokenKind::Whitespace),
    (r"^([[:alpha:]_][[:word:]]*)", PPTokenKind::Identifier), // TODO unicode
    (
        r"^\.?[0-9](([eEpP][\+\-])|[[:word:]]|\.)*",
        PPTokenKind::PPNumber,
    ),
    (
        concat!(
            r"^[LuU]?",  // type specifiers
            r"'",        // opening single quote
            r"(",        // open regex group
            r"[^'\\\n]", // anything except single quote, backslash, or newline
            r"|(\\')",   // match nested single quote
            r"|(\\)",    // match backslash to start any escape
            r")*?",      // close regex group and multiples (non greedy)
            r"'",        // closing single quote
        ),
        PPTokenKind::CharacterConstant,
    ),
    (
        concat!(
            r"^(u8|u|U|L)?", // type specifiers
            "\"",            // opening double quote
            r"(",            // open regex group
            r#"[^"\\\n]"#,   // anything except double quote, backslash, or newline
            r#"|(\\")"#,     // match nested double quote
            r"|(\\)",        // match backslash to start any escape
            r")*?",          // close regex group and multiples (non greedy)
            "\"",            // closing double quote
        ),
        PPTokenKind::StringLiteral,
    ),
    (
        concat!(
            // regex alternations (the vertical bar syntax) prefer to match the
            // pattern on the left, so in effect the regex "ab|abc|c" will
            // always have two matches of "ab" and "c" rather than one match of
            // "abc". Thus, we carefully order the patterns such that the longer
            // operators are first. This has no runtime performance cost. For a
            // more legible list of these operators, see
            // test::test_phase3_punctuator().
            r"^((\[)|(\])|(\()|(\))|(\{)|(\})|(\->)|(\+\+)|(\-\-)|(<=)|(>=)",
            r"|(==)|(!=)|(\&\&)|(\|\|)|(\?)|(;)|(\.\.\.)|(\*=)|(/=)|(%=)",
            r"|(\+=)|(\-=)|(<<=)|(>>=)|(\&=)|(\^=)|(\|=)|(,)|(\#\#)|(\#)|(<:)",
            r"|(:>)|(<%)|(%>)|(%:%:)",
            // the shorter operators that have to be deprioritized to allow the
            // longer ones a chance to match
            r"|(%:)|(<<)|(>>)",
            r"|(<)|(>)|(!)|(:)|(\&)|(\*)|(\+)|(\-)|(\~)|(/)|(%)|(\^)|(\|)|(=)|(\.))",
        ),
        PPTokenKind::Punctuator,
    ),
];

lazy_static! {
    static ref REGEX_SET: RegexSet = RegexSet::new(TOKEN_PATTERNS.iter().map(|a| a.0)).unwrap();
    static ref REGEXS: Vec<Regex> = TOKEN_PATTERNS
        .iter()
        .map(|a| Regex::new(a.0).unwrap())
        .collect();
}

fn find_match(input: &str, index: usize) -> &str {
    let regex = &REGEXS[index];
    regex.find(input).unwrap().as_str()
}

/// Categorize the first token of the input string
///
/// Returns the slice containing the entire token plus its kind. This slice may
/// be less than the input string if the input lexes as more than one token.
///
/// The input must be non-empty.
pub fn lex_one_token(input: &str) -> (&str, PPTokenKind) {
    // choose longest match
    let mut matches: Vec<(&str, usize)> = REGEX_SET
        .matches(input)
        .iter()
        .map(|i| (find_match(input, i), i)) // extract substring by rerunning regex
        .collect::<Vec<_>>();

    assert_ne!(matches.len(), 0);

    // sort by length of match, breaking ties by choosing rules listed later
    // in TOKEN_PATTERNS
    matches.sort_by_key(|(s, i)| (s.len(), *i));

    let &(slice, index) = matches.last().unwrap();
    let kind = TOKEN_PATTERNS[index].1;

    (slice, kind)
}

/// Test if all tokens resulting from lexer have the correct input
fn test_correct_input(tokens: &[PPToken], input: u32) -> bool {
    tokens.iter().all(|t| match t.origin {
        TokenOrigin::Source(span) => span.pos.input == input,
        TokenOrigin::Macro(..) => unreachable!(),
    })
}

/// Categorize all tokens given by the input token sequence
pub fn lex(tuctx: &mut TUCtx, tokens: Vec<CharToken>, input: Rc<Input>) -> Vec<PPToken> {
    let string = CharToken::to_string(&tokens);
    debug_assert_eq!(tokens.len(), string.len());

    let mut i = 0;
    let mut output = Vec::new();

    while i < string.len() {
        trace!("lex() i={:?} string[i..]={:?}", i, &string[i..]);
        let (slice, kind) = lex_one_token(&string[i..]);
        debug!("lex() slice={:?} kind={:?}", slice, kind);

        let len = slice.len();
        let first = &tokens[i];
        let last = &tokens[i + len - 1];
        i += len;

        if kind == PPTokenKind::Other && slice.starts_with("'") {
            // A properly terminated string would've matched the StringLiteral
            // regex, thus we know this string is unterminated

            // TODO move to phase7
            tuctx.emit_message(
                first.span,
                MessageKind::Phase3MissingTerminator { terminator: '\'' },
            );

            // skip ahead
            // where should we stop? newline?
            while i < string.len() && tokens[i].value != '\n' {
                i += 1;
            }
        } else {
            // CharTokens may have length greater than one because of trigraphs
            let mut span = first.span;
            span.len = last.span.pos.absolute + last.span.len - first.span.pos.absolute;

            output.push(PPToken {
                kind,
                value: slice.to_owned(),
                origin: TokenOrigin::Source(span),
            })
        }
    }

    debug_assert!(Rc::ptr_eq(&tuctx.inputs[input.id as usize], &input));

    output.push(PPToken {
        kind: PPTokenKind::EndOfFile,
        value: "".to_owned(),
        origin: TokenOrigin::Source(match output.last().map(|t| &t.origin) {
            Some(TokenOrigin::Source(last_span)) => *last_span,
            // Some(TokenOrigin::Source(last_span)) => {
            //     let mut eof_span = *last_span;
            //     eof_span.pos.absolute += last_span.len;
            //     eof_span
            // },
            Some(TokenOrigin::Macro(..)) => unreachable!(),
            None => TextSpan {
                pos: TextPosition {
                    input: input.id,
                    absolute: 1,
                },
                len: 0,
            },
        }),
    });

    debug_assert!(test_correct_input(&output, input.id));

    if log_enabled!(log::Level::Trace) {
        for (i, token) in output.iter().enumerate() {
            trace!("lex() output[{}] = {:?}", i, token);
        }
    }

    output
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::front::c::message::Message;

    fn phase3(input: &str) -> (Vec<PPToken>, Vec<Message>) {
        let session = crate::Session::builder()
            .parse_cli_args_from_str(&[
                "--pass=state_read_input",
                "--pass=phase1",
                "--pass=phase2",
                "--pass=phase3",
                "--pass=state_save(pptokens)",
            ])
            .unwrap()
            .build();
        let mut tu = crate::tu::CTranslationUnit::builder(&session)
            .source_string("<unit-test>", input)
            .build();
        tu.run().unwrap();

        let output = tu.saved_states("pptokens")[0]
            .clone()
            .into_pptokens()
            .unwrap();
        let messages = tu.messages().to_vec();

        (output, messages)
    }

    // StringLiteral is same basically
    #[test]
    fn test_phase3_characterconstant() {
        fn case(input: &str) {
            let (tokens, _) = phase3(input);
            dbg!(&tokens);
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].kind, PPTokenKind::CharacterConstant);
            assert_eq!(tokens[0].as_str(), input);
            assert_eq!(tokens[1].kind, PPTokenKind::EndOfFile);
        }

        case("'a'");
        case("L'a'");
        case("u'a'");
        case("U'a'");

        case("'abc'");
        case("'<=>'");

        case(r"'\'\''");

        let (tokens, _) = phase3("'a' + 'b'");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].as_str(), "'a'");
        assert_eq!(tokens[4].as_str(), "'b'");
    }

    #[test]
    fn test_phase3_whitespace() {
        fn case(input: &str) {
            let (tokens, _) = phase3(input);
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].kind, PPTokenKind::Whitespace);
            assert_eq!(tokens[0].as_str(), input);
            assert_eq!(tokens[1].kind, PPTokenKind::EndOfFile);
        }

        case(" ");
        case("\n");
        case("\x0c"); // form feed
        case("\r");
        case("\t");
        case("\x0b"); // vertical tab

        case("//comment");

        // newline is excluded from comment whitespace
        let (tokens, _) = phase3("//comment\n");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, PPTokenKind::Whitespace);
        assert_eq!(tokens[1].kind, PPTokenKind::Whitespace);
        assert_eq!(tokens[2].kind, PPTokenKind::EndOfFile);

        let (tokens, _) = phase3("/* comment */\n");
        assert_eq!(tokens.len(), 3);

        let (tokens, _) = phase3("test /* whitespace */");
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0].value, "test");
        assert_eq!(tokens[0].kind, PPTokenKind::Identifier);
        assert_eq!(tokens[1].kind, PPTokenKind::Whitespace);
        assert_eq!(tokens[2].value, "/* whitespace */");
        assert_eq!(tokens[2].kind, PPTokenKind::Whitespace);

        let (tokens, _) = phase3("/* \n */");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind, PPTokenKind::Whitespace);
        assert_eq!(tokens[1].kind, PPTokenKind::EndOfFile);

        // TODO other comment examples in 6.4.9
    }

    #[test]
    fn test_phase3_ppnumber() {
        fn case(input: &str) {
            let (tokens, _) = phase3(input);
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].kind, PPTokenKind::PPNumber);
            assert_eq!(tokens[0].as_str(), input);
            assert_eq!(tokens[1].kind, PPTokenKind::EndOfFile);
        }

        case("0123456789");
        case("01234.56789");
        case(".0123456789.");
        case(".01234abc_def56789.");
        case("0e-");
        case("0P+.");
    }

    #[test]
    fn test_phase3_identifier() {
        fn case(input: &str) {
            let (tokens, _) = phase3(input);
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].kind, PPTokenKind::Identifier);
            assert_eq!(tokens[0].as_str(), input);
            assert_eq!(tokens[1].kind, PPTokenKind::EndOfFile);
        }

        case("a");
        case("aZas_0234");
    }

    #[test]
    fn test_phase3_punctuator() {
        fn case(input: &str) {
            let (tokens, _) = phase3(input);
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].kind, PPTokenKind::Punctuator);
            assert_eq!(tokens[0].as_str(), input);
            assert_eq!(tokens[1].kind, PPTokenKind::EndOfFile);
        }

        static PUNCTUATORS: &[&'static str] = &[
            "[", "]", "(", ")", "{", "}", ".", "->", "++", "--", "&", "*", "+", "-", "~", "!", "/",
            "%", "<<", ">>", "<", ">", "<=", ">=", "==", "!=", "^", "|", "&&", "||", "?", ":", ";",
            "...", "=", "*=", "/=", "%=", "+=", "-=", "<<=", ">>=", "&=", "^=", "|=", ",", "##",
            "#", "<:", ":>", "<%", "%>", "%:", "%:%:",
        ];
        for punctuator in PUNCTUATORS {
            case(punctuator);
        }
    }

    // TODO test strings
}
