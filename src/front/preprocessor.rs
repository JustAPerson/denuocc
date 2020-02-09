// Copyright (C) 2019 Jason Priest
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either  version 3 of the  License, or (at your  option) any later
// version.
//
// This program is distributed  in the hope that it will  be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR  A PARTICULAR  PURPOSE.  See  the GNU  General  Public  License for  more
// details.
//
// You should have received a copy of  the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

//! Phase 4: Execute preprocessor directives
use std::collections::HashMap;
use std::rc::Rc;
use std::vec::IntoIter;

use log::trace;

use crate::front::lexer::lex_one_token;
use crate::message::{MessageKind, MessagePart};
use crate::token::{self, Location, PPToken, PPTokenKind};
use crate::tu::TUCtx;

type Line = Vec<PPToken>;

#[derive(Clone, Debug)]
struct MacroObject {
    name: String,
    replacement: Vec<PPToken>,
    location: token::Location,
}

#[derive(Clone, Debug)]
struct MacroFunction {
    name: String,
    replacement: Vec<PPToken>,
    params: Vec<String>,
    vararg: bool,
    location: token::Location,
}

#[derive(Clone, Debug)]
enum MacroDef {
    Object(MacroObject),
    Function(MacroFunction),
}

impl MacroDef {
    pub fn name(&self) -> &str {
        match self {
            MacroDef::Object(object) => &object.name,
            MacroDef::Function(func) => &func.name,
        }
    }

    pub fn location(&self) -> &token::Location {
        match self {
            MacroDef::Object(object) => &object.location,
            MacroDef::Function(func) => &func.location,
        }
    }

    pub fn as_function(&self) -> &MacroFunction {
        match self {
            MacroDef::Function(func) => &func,
            _ => panic!(),
        }
    }

    fn equivalent(&self, other: &Self) -> bool {
        // Compare two sequences of tokens
        //
        // White space is kinda significant. The whitespace at beginning or end
        // is insignificant, but internal whitespace matters (but consider
        // multiple adjacent whitespace tokens as 1)
        fn compare_tokens(lhs: &[PPToken], rhs: &[PPToken]) -> bool {
            let mut lhs = tokens_trim_whitespace(lhs).iter().peekable();
            let mut rhs = tokens_trim_whitespace(rhs).iter().peekable();

            while let (Some(left), Some(right)) = (lhs.next(), rhs.next()) {
                if left.kind != right.kind {
                    return false
                }
                if left.kind == PPTokenKind::Whitespace {
                    // both left and right are whitespace
                    // now we don't care how many adjacent whitespace tokens there are

                    while lhs.peek().map(|t| t.is_whitespace()) == Some(true) {
                        lhs.next();
                    }
                    while rhs.peek().map(|t| t.is_whitespace()) == Some(true) {
                        rhs.next();
                    }
                } else if left.value != right.value {
                    return false
                }
            }

            // sequences are equal if we have consumed both in their entirety
            lhs.next().is_none() && rhs.next().is_none()
        }

        match (self, other) {
            (
                MacroDef::Object(MacroObject {
                    replacement: orig_rep,
                    ..
                }),
                MacroDef::Object(MacroObject {
                    replacement: other_rep,
                    ..
                }),
            ) => compare_tokens(orig_rep, other_rep),
            (
                MacroDef::Function(MacroFunction {
                    params: orig_params,
                    vararg: orig_vararg,
                    replacement: orig_rep,
                    ..
                }),
                MacroDef::Function(MacroFunction {
                    params: other_params,
                    vararg: other_vararg,
                    replacement: other_rep,
                    ..
                }),
            ) => {
                orig_params == other_params
                    && orig_vararg == other_vararg
                    && compare_tokens(orig_rep, other_rep)
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
enum IfCondition {
    Plain(Line),
    Defined(PPToken),
    Undefined(PPToken),
    Empty,
}

impl IfCondition {
    pub fn evaluate(&self, defines: &HashMap<String, MacroDef>) -> bool {
        match self {
            IfCondition::Plain(_line) => unimplemented!(),
            IfCondition::Defined(token) => defines.contains_key(&token.value),
            IfCondition::Undefined(token) => !defines.contains_key(&token.value),

            // only ever used when discarding the output in order to better recover from parsing
            // errors
            IfCondition::Empty => unreachable!(),
        }
    }
}

#[derive(Debug)]
enum Directive {
    IfSection {
        condition: IfCondition,
        main_body: Vec<Line>,
        elifs: Vec<(IfCondition, Vec<Line>)>,
        else_body: Option<Vec<Line>>,
    },
    Define(MacroDef),
    Undefine(PPToken),
    Text(Vec<PPToken>),
}

/// Checks whether this is the last line of the file
///
/// This line would is empty except for the EndOfFile token
fn line_is_eof(line: &[PPToken]) -> bool {
    line[0].kind == PPTokenKind::EndOfFile
}

/// Determines if this line of tokens signifies a directive
///
/// This allows for leading whitespace as well as whitespace between the # and
/// the directive name
fn line_is_directive(line: &[PPToken]) -> Option<&str> {
    let mut iter = line.iter().filter(|t| !t.is_whitespace());
    let first = iter.next();
    let second = iter.next();

    if first.map(|t| t.as_str()) != Some("#") {
        return None;
    }
    if second.map(|t| t.kind) != Some(PPTokenKind::Identifier) {
        return None;
    }

    second.map(|t| t.as_str())
}

/// Returns the token of the name of the directive
fn line_get_directive_name(line: &[PPToken]) -> &PPToken {
    debug_assert!(line_is_directive(&line).is_some());
    dbg!(line);

    line.iter().filter(|t| !t.is_whitespace()).nth(1).unwrap()
}

/// Collect lines until first directive and append them to `output`
fn collect_lines_until_directive(line_iter: &mut IntoIter<Line>, output: &mut Vec<PPToken>) {
    while line_iter.as_slice().len() > 0 {
        let line = &line_iter.as_slice()[0];
        if line_is_eof(line) || line_is_directive(line).is_some() {
            break;
        }

        let mut line = line_iter.next().unwrap();
        output.append(&mut line);
    }
}

/// Collect lines belonging to an if body
fn collect_lines_until_ify_directive(line_iter: &mut IntoIter<Line>) -> Vec<Line> {
    let mut output = Vec::new();
    let mut depth = 0;

    loop {
        let line = &line_iter.as_slice()[0];
        if line_is_eof(line) {
            break;
        }

        let directive = line_is_directive(line);
        match (depth, directive) {
            // we want to collect the body of an if directive, which is
            // delimited by one of the following directives, which may possibly
            // nest
            (0, Some("else")) => break,
            (0, Some("elif")) => break,
            (0, Some("endif")) => break,

            (_, Some("if")) => depth += 1,
            (_, Some("endif")) => depth -= 1,

            (_, _) => {}
        }

        output.push(line_iter.next().unwrap())
    }

    output
}

/// Consumes from Iterator up to and including the directive name
///
/// Note: there will always be at least one remaining token in the iterator
/// because every line ends in a newline whitespace token.
fn line_skip_until_directive_content(iter: &mut IntoIter<PPToken>) {
    debug_assert!(line_is_directive(iter.as_slice()).is_some());

    iter.filter(|t| !t.is_whitespace()).nth(1).unwrap();
}

fn line_peek(iter: &mut IntoIter<PPToken>) -> Option<&PPToken> {
    iter.as_slice().get(0)
}

fn line_skip_whitespace_until_newline(iter: &mut IntoIter<PPToken>) {
    while line_peek(iter).map(|t| t.is_whitespace_not_newline()) == Some(true) {
        iter.next().unwrap();
    }
}

fn tokens_trim_whitespace(tokens: &[PPToken]) -> &[PPToken] {
    if tokens.is_empty() {
        return tokens;
    }

    let mut first = 0;
    let mut last = tokens.len() - 1;
    while first < tokens.len() && tokens[first].kind == PPTokenKind::Whitespace {
        first += 1;
    }
    while last >= first && tokens[last].kind == PPTokenKind::Whitespace {
        last -= 1;
    }
    &tokens[first..last + 1]
}

/// verify that the remainder of line is only an identifier and newline, with
/// optional whitespace in between
fn line_get_identifier_and_newline(
    tuctx: &mut TUCtx,
    token_iter: &mut IntoIter<PPToken>,
) -> Option<PPToken> {
    // first remaining non-whitespace token is ident
    let identifier = token_iter
        .skip_while(PPToken::is_whitespace_not_newline)
        .next()
        .unwrap();
    if identifier.kind != PPTokenKind::Identifier {
        tuctx.emit_message(
            identifier.location,
            MessageKind::ExpectedFound {
                expected: MessagePart::Plain("identifier".to_owned()),
                found: MessagePart::PPToken(identifier.kind),
            },
        );
        return None;
    }

    // check that nothing else comes after identifier
    let newline = token_iter
        .skip_while(PPToken::is_whitespace_not_newline)
        .next()
        .unwrap();
    if newline.as_str() != "\n" {
        tuctx.emit_message(
            newline.location,
            MessageKind::ExpectedFound {
                expected: MessagePart::Plain("newline".to_owned()),
                found: MessagePart::PPToken(newline.kind),
            },
        );
    }

    Some(identifier)
}

fn parse_directive_define(tuctx: &mut TUCtx, tokens: Vec<PPToken>) -> Option<Directive> {
    let mut token_iter = tokens.into_iter();
    line_skip_until_directive_content(&mut token_iter);
    line_skip_whitespace_until_newline(&mut token_iter);

    let name_token = token_iter.next().unwrap();
    if name_token.kind != PPTokenKind::Identifier {
        tuctx.emit_message(
            name_token.location,
            MessageKind::ExpectedFound {
                expected: MessagePart::PPToken(PPTokenKind::Identifier),
                found: MessagePart::PPToken(name_token.kind),
            },
        );

        return None;
    }

    if line_peek(&mut token_iter).unwrap().as_str() == "(" {
        token_iter.next().unwrap();

        let mut vararg = false;
        let mut params = Vec::new();

        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        enum State {
            LParen,
            Comma,
            Ident,
            Vararg,
        }
        let mut state = State::LParen;

        while let Some(token) = token_iter.next() {
            match (state, token.kind, token.as_str()) {
                (State::LParen, _, ")") | (State::Ident, _, ")") | (State::Vararg, _, ")") => break,

                (_, PPTokenKind::Whitespace, _) => continue,

                (State::LParen, PPTokenKind::Identifier, ..)
                | (State::Comma, PPTokenKind::Identifier, ..) => {
                    state = State::Ident;
                    if !params.contains(&token.value) {
                        params.push(token.value);
                    } else {
                        tuctx.emit_message(
                            token.location,
                            MessageKind::Phase4RepeatedMacroParameter {
                                parameter: token.value,
                            },
                        );
                    }
                }
                (State::LParen, _, "...") | (State::Comma, _, "...") => {
                    state = State::Vararg;
                    vararg = true;
                }
                (State::LParen, ..) | (State::Comma, ..) => {
                    tuctx.emit_message(
                        token.location,
                        MessageKind::ExpectedFound {
                            expected: MessagePart::Plain("identifier or `...`".to_owned()),
                            found: MessagePart::Plain(format!("`{}`", token.value)),
                        },
                    );

                    return None;
                }

                (State::Ident, _, ",") => {
                    state = State::Comma;
                }
                (State::Ident, ..) => {
                    tuctx.emit_message(
                        token.location,
                        MessageKind::ExpectedFound {
                            expected: MessagePart::Plain("`,`".to_owned()),
                            found: MessagePart::Plain(format!("`{}`", token.value)),
                        },
                    );

                    return None;
                }

                // closing paren handled by first pattern in match
                // so we've encountered something after `...` which
                // is erroneous
                (State::Vararg, ..) => {
                    tuctx.emit_message(
                        token.location,
                        MessageKind::ExpectedFound {
                            expected: MessagePart::Plain("`)`".to_owned()),
                            found: MessagePart::Plain(format!("`{}`", token.value)),
                        },
                    );

                    return None;
                }
            }
        }

        // do not trim whitespace yet, we want to keep EOL token when detecting
        // `#` at end of line
        let replacement = token_iter.as_slice();

        // Ensure # is followed by a param
        let mut singlehash: Option<&Location> = None;
        for token in replacement
            .iter()
            .filter(|t| !t.is_whitespace_not_newline())
        {
            if let Some(location) = singlehash {
                if !(params.contains(&token.value) || (vararg && token.value == "__VA_ARGS__")) {
                    tuctx.emit_message(location.clone(), MessageKind::Phase4IllegalSingleHash);
                    return None;
                }
                singlehash = None;
            } else if token.as_str() == "#" {
                singlehash = Some(&token.location);
            }
        }

        // Now remove whitespace at beginning/end of replacement because it
        // simplifies testing for `##` (but it's also how macros are supposed to
        // expand)
        let replacement = tokens_trim_whitespace(replacement).to_vec();

        // Test for ## at begin/end of macro
        if replacement.len() > 0 {
            let mut doublehash = None;
            if replacement.first().unwrap().as_str() == "##" {
                doublehash = replacement.first();
            } else if replacement.last().unwrap().as_str() == "##" {
                doublehash = replacement.last();
            }

            if let Some(token) = doublehash {
                tuctx.emit_message(token.location.clone(), MessageKind::Phase4IllegalDoubleHash);
                return None;
            }
        }

        Some(Directive::Define(MacroDef::Function(MacroFunction {
            name: name_token.value,
            params,
            vararg,
            replacement,
            location: name_token.location,
        })))
    } else {
        Some(Directive::Define(MacroDef::Object(MacroObject {
            name: name_token.value,
            replacement: tokens_trim_whitespace(token_iter.as_slice()).to_vec(),
            location: name_token.location,
        })))
    }
}

fn parse_directive_undefine(tuctx: &mut TUCtx, tokens: Vec<PPToken>) -> Option<Directive> {
    let mut token_iter = tokens.into_iter();
    line_skip_until_directive_content(&mut token_iter);
    line_skip_whitespace_until_newline(&mut token_iter);

    let name_token = token_iter.next().unwrap();
    if name_token.kind != PPTokenKind::Identifier {
        tuctx.emit_message(
            name_token.location,
            MessageKind::ExpectedFound {
                expected: MessagePart::PPToken(PPTokenKind::Identifier),
                found: MessagePart::PPToken(name_token.kind),
            },
        );

        return None;
    }

    line_skip_whitespace_until_newline(&mut token_iter);

    if line_peek(&mut token_iter).unwrap().as_str() == "\n" {
        return Some(Directive::Undefine(name_token));
    } else {
        tuctx.emit_message(
            name_token.location,
            MessageKind::ExpectedFound {
                expected: MessagePart::Plain("newline".to_owned()),
                found: MessagePart::PPToken(name_token.kind),
            },
        );
        return None;
    }
}

fn parse_directive_if_generic(
    tuctx: &mut TUCtx,
    condition: IfCondition,
    line_iter: &mut IntoIter<Vec<PPToken>>,
    output: &mut Vec<Directive>,
) {
    let mut main_body = None;
    let mut elifs = Vec::new();
    let mut else_body = None;

    #[derive(Debug)]
    enum State {
        Main,
        Elif(IfCondition),
        Else,
    }

    let mut state = State::Main;
    loop {
        let body = collect_lines_until_ify_directive(line_iter);
        let line = line_iter.next().unwrap();
        if line_is_eof(&line) {
            tuctx.emit_message(
                line[0].location.clone(),
                MessageKind::ExpectedFound {
                    expected: MessagePart::Directive("endif".to_owned()),
                    found: MessagePart::PPToken(PPTokenKind::EndOfFile),
                },
            );
            return;
        }

        // update directive variables
        match &state {
            State::Main => main_body = Some(body),
            State::Elif(c) => elifs.push((c.clone(), body)),
            State::Else => else_body = Some(body),
        }

        // update state
        match (state, line_is_directive(&line)) {
            (_, Some("endif")) => break,

            // next directive is elif
            (State::Main, Some("elif")) | (State::Elif(..), Some("elif")) => {
                // skip hash and directive name
                let mut iter = line.into_iter();
                line_skip_until_directive_content(&mut iter);

                let condition = IfCondition::Plain(iter.collect());
                state = State::Elif(condition);
            }

            // next directive is `else`
            (State::Main, Some("else")) | (State::Elif(..), Some("else")) => {
                // skip hash and directive name
                let mut iter = line.into_iter();
                line_skip_until_directive_content(&mut iter);

                let should_be_newline = iter.next().unwrap();
                if should_be_newline.as_str() != "\n" {
                    tuctx.emit_message(
                        should_be_newline.location,
                        MessageKind::ExpectedFound {
                            expected: MessagePart::Plain("newline".to_owned()),
                            found: MessagePart::PPToken(should_be_newline.kind),
                        },
                    );
                }

                state = State::Else;
            }

            // `else` directive should be followed by an `endif` directive. That
            // case is handled above, so if we reach this case, the next
            // directive is either `else` or `elif`, both of which would be
            // invalid.
            (State::Else, Some(directive)) => {
                tuctx.emit_message(
                    line[0].location.clone(),
                    MessageKind::ExpectedFound {
                        expected: MessagePart::Directive("endif".to_owned()),
                        found: MessagePart::Directive(directive.to_owned()),
                    },
                );
                return;
            }

            _ => unreachable!(),
        }
    }

    output.push(Directive::IfSection {
        condition: condition,
        main_body: main_body.unwrap(),
        elifs,
        else_body,
    })
}

fn parse_directive_if(
    tuctx: &mut TUCtx,
    line: Vec<PPToken>,
    line_iter: &mut IntoIter<Vec<PPToken>>,
    output: &mut Vec<Directive>,
) {
    // collect everything after directive name
    let mut token_iter = line.into_iter();
    line_skip_until_directive_content(&mut token_iter);
    let condition = IfCondition::Plain(token_iter.collect());

    parse_directive_if_generic(tuctx, condition, line_iter, output);
}

fn parse_directive_ifdef(
    tuctx: &mut TUCtx,
    line: Vec<PPToken>,
    line_iter: &mut IntoIter<Vec<PPToken>>,
    output: &mut Vec<Directive>,
) {
    // skip `#ifdef`
    let mut token_iter = line.into_iter();
    line_skip_until_directive_content(&mut token_iter);
    // verify that all that remains is a identifier
    let identifier = line_get_identifier_and_newline(tuctx, &mut token_iter);

    if let Some(identifier) = identifier {
        let condition = IfCondition::Defined(identifier);
        parse_directive_if_generic(tuctx, condition, line_iter, output);
    } else {
        // in the event we fail to parse the identifier, continue parsing the
        // #else and #endif directives to reduce incorrect errors

        // mutate iterator but discard output
        parse_directive_if_generic(tuctx, IfCondition::Empty, line_iter, &mut Vec::new());
    }
}

fn parse_directive_ifndef(
    tuctx: &mut TUCtx,
    line: Vec<PPToken>,
    line_iter: &mut IntoIter<Vec<PPToken>>,
    output: &mut Vec<Directive>,
) {
    // skip `#ifdef`
    let mut token_iter = line.into_iter();
    line_skip_until_directive_content(&mut token_iter);
    // verify that all that remains is a identifier
    let identifier = line_get_identifier_and_newline(tuctx, &mut token_iter);

    if let Some(identifier) = identifier {
        let condition = IfCondition::Undefined(identifier);
        parse_directive_if_generic(tuctx, condition, line_iter, output);
    } else {
        // in the event we fail to parse the identifier, continue parsing the
        // #else and #endif directives to reduce incorrect errors

        // mutate iterator but discard output
        parse_directive_if_generic(tuctx, IfCondition::Empty, line_iter, &mut Vec::new());
    }
}

/// Breaks stream into separate lines
fn parse_lines(mut tokens: Vec<PPToken>) -> Vec<Line> {
    // Insert an extra newline into file to simplify parsing of directives
    let len = tokens.len();
    let location;

    debug_assert_ne!(len, 0);
    if len > 1 {
        // clone location of character before EOF
        location = tokens[len - 2].location.clone()
    } else {
        // use EOF to get location for newline
        debug_assert_eq!(tokens[0].kind, PPTokenKind::EndOfFile);
        let input = match &tokens[0].location {
            token::Location::Direct(loc) => &loc.input,
            token::Location::Indirect(..) => unreachable!(),
        };

        location = token::Location::Direct(token::DirectLocation {
            input: Rc::clone(input),
            begin: token::Position::default(),
            len: 1,
        })
    };

    // insert before EOF
    tokens.insert(
        len - 1,
        PPToken {
            kind: PPTokenKind::Whitespace,
            value: "\n".to_owned(),
            location,
        },
    );
    debug_assert_eq!(tokens.last().unwrap().kind, PPTokenKind::EndOfFile);

    let mut token_iter = tokens.into_iter();
    let mut lines: Vec<Vec<PPToken>> = Vec::new();
    lines.push(Vec::new());

    while let Some(token) = token_iter.next() {
        // don't discard newline
        // necessary for correctly serializing back into plaintext
        let was_newline = token.as_str() == "\n";

        lines.last_mut().unwrap().push(token);

        if was_newline {
            lines.push(Vec::new());
        }
    }
    // every line is non empty
    debug_assert!(lines.iter().all(|line| line.len() > 0));
    // every line (but last) ends in newline
    debug_assert!(lines
        .iter()
        .take(lines.len() - 1)
        .all(|line| line.last().unwrap().as_str() == "\n"));
    // last line is only EOF
    debug_assert_eq!(lines.last().unwrap().len(), 1);
    debug_assert_eq!(lines.last().unwrap()[0].kind, PPTokenKind::EndOfFile);

    lines
}

/// Collates lines into directives
fn parse_directives(tuctx: &mut TUCtx, lines: Vec<Line>) -> Vec<Directive> {
    let mut directives = Vec::<Directive>::new();
    let mut line_iter = lines.into_iter();
    while let Some(line) = line_iter.next() {
        if line_is_eof(&line) {
            break;
        }

        match line_is_directive(&line) {
            Some("define") => {
                if let Some(directive) = parse_directive_define(tuctx, line) {
                    directives.push(directive)
                }
            }
            Some("undef") => {
                if let Some(directive) = parse_directive_undefine(tuctx, line) {
                    directives.push(directive)
                }
            }
            Some("if") => parse_directive_if(tuctx, line, &mut line_iter, &mut directives),
            Some("ifdef") => parse_directive_ifdef(tuctx, line, &mut line_iter, &mut directives),
            Some("ifndef") => parse_directive_ifndef(tuctx, line, &mut line_iter, &mut directives),

            // complain about invalid directive
            Some(directive) => {
                tuctx.emit_message(
                    line_get_directive_name(&line).location.clone(),
                    MessageKind::Phase4InvalidDirective {
                        directive: directive.to_owned(),
                    },
                );
            }

            // No directive means it's text
            None => {
                let mut text = line;
                collect_lines_until_directive(&mut line_iter, &mut text);

                directives.push(Directive::Text(text))
            }
        }
    }

    directives
}

/// Process if-sections
fn stage_one(tuctx: &mut TUCtx, lines: Vec<Line>) -> Vec<Directive> {
    let input_directives = parse_directives(tuctx, lines);

    let mut output_directives = Vec::new();
    let mut defines = HashMap::new();

    'outer: for directive in input_directives {
        // split this from the following match so that we can consume the
        // directive without cloning its contents.
        if let Directive::IfSection {
            condition,
            main_body,
            elifs,
            else_body,
        } = &directive
        {
            if condition.evaluate(&mut defines) {
                output_directives.append(&mut stage_one(tuctx, main_body.clone()));
                continue 'outer;
            }
            for (condition, body) in elifs {
                if condition.evaluate(&mut defines) {
                    output_directives.append(&mut stage_one(tuctx, body.clone()));
                    continue 'outer;
                }
            }
            if let Some(else_body) = else_body.clone() {
                output_directives.append(&mut stage_one(tuctx, else_body));
            }
            continue 'outer;
        }

        match &directive {
            // IfSection is handled above to avoid cloning
            Directive::IfSection { .. } => unreachable!(),

            Directive::Define(macrodef) => {
                defines.insert(macrodef.name().to_owned(), macrodef.clone());
            }
            Directive::Undefine(name) => {
                defines.remove(&name.value);
            }
            Directive::Text(..) => {}
        }

        output_directives.push(directive);
    }

    output_directives
}

fn disable_macro_recursion(tokens: &mut Vec<PPToken>, name: &PPToken) {
    for token in tokens {
        if token.value == name.value {
            // The contents of the macro expansion included the macro's own
            // name. Mark that this macro cannot be expanded
            token.kind = PPTokenKind::IdentifierNonExpandable;
        }
    }
}

/// Escape string literals and character constants
///
/// We wish to place these tokens within a string literal, so we only care about
/// escaping backslashes and double quotes. In either case, we prepend those
/// characters with another backslash.
fn escape(output: &mut String, token: &PPToken) {
    for c in token.value.chars() {
        match c {
            '\\' => {
                output.push('\\');
                output.push('\\');
            }
            '"' => {
                output.push('\\');
                output.push('"');
            }
            c => output.push(c),
        }
    }
}

fn stringize(input: &[PPToken], location: Location) -> PPToken {
    use PPTokenKind::*;

    let mut output = String::new();
    let inner = tokens_trim_whitespace(input);

    // A string literal must include its opening/closing quotations
    output.push('"');

    for token in inner {
        trace!("stringize() token={:?}", &token);
        match token.kind {
            Whitespace => {
                // sequences of multiple whitespace tokens should be replaced
                // with only one space character.
                if output.chars().next_back() != Some(' ') {
                    output.push(' ');
                }
            }
            StringLiteral | CharacterConstant => escape(&mut output, &token),
            _ => output.push_str(&token.value),
        }
    }

    output.push('"');

    trace!("stringize() output={:?}", &output);

    PPToken {
        kind: PPTokenKind::StringLiteral,
        value: output,
        location,
    }
}

/// Struct for managing complex expansion logic
///
/// This largely follows the algorithm proposed in X3J11/86-196, an ancient
/// document from the ANSI C commitee back in 1986. You can find updated version
/// on google.
struct Expander<'tu, 'drv, 'def> {
    /// Context for reporting errors
    tuctx: &'tu mut TUCtx<'drv>,

    /// Macro definitions
    ///
    /// Stored as a pointer so we can easily recurse with a second Expander to
    /// handle small subsets of the token stream.
    defines: &'def mut HashMap<String, Rc<MacroDef>>,

    /// Output of expansion
    ///
    /// Modified by multiple functions while running `expand()`.
    output: Vec<PPToken>,

    /// Tokens to be rescanned after expansion
    ///
    /// These tokens are examined before any of the remaining tokens that
    /// actually derive from the input file. This vector is stored in reverse
    /// order. We want a collection that we can easily append to and pop from
    /// the front of. This is easily achieved by calling `.reverse()` on data
    /// before appending it to this.
    rescan: Vec<PPToken>,

    /// Tokens of the current text line of the input file
    ///
    /// When this is empty, directives will be processed until a text line is
    /// found.
    line: Option<IntoIter<PPToken>>,

    /// Lines of the actual input file
    directives: IntoIter<Directive>,
}

impl<'tu, 'drv, 'def> Expander<'tu, 'drv, 'def> {
    /// Construct new expander from a whole file
    fn from_directives(
        tuctx: &'tu mut TUCtx<'drv>,
        defines: &'def mut HashMap<String, Rc<MacroDef>>,
        directives: Vec<Directive>,
    ) -> Self {
        Self {
            tuctx,
            defines,
            output: Vec::new(),

            rescan: Vec::new(),
            line: None,
            directives: directives.into_iter(),
        }
    }

    /// Construct new expander from a single line of text
    fn from_tokens(
        tuctx: &'tu mut TUCtx<'drv>,
        defines: &'def mut HashMap<String, Rc<MacroDef>>,
        line: Vec<PPToken>,
    ) -> Self {
        Self {
            tuctx,
            defines,
            output: Vec::new(),

            rescan: Vec::new(),
            line: if line.is_empty() {
                None
            } else {
                Some(line.into_iter())
            },
            directives: Vec::new().into_iter(),
        }
    }

    /// Rescan these tokens
    ///
    /// The following call to `next_token()` will return the first element of
    /// this vector
    fn rescan(&mut self, mut tokens: Vec<PPToken>) {
        // self.rescan is stored in reverse order so that we can easily append
        // to and pop from it.
        tokens.reverse();
        self.rescan.append(&mut tokens);
    }

    /// Retrieve next token from `self.line` and cleanup when we've exhausted it
    fn next_token_from_line(&mut self) -> PPToken {
        // We assume this is only called when `self.line.is_some()` AND we
        // assume that `self.line` always starts non-empty (because every line
        // ends with a newline token)
        let line = self.line.as_mut().unwrap();
        let token = line.next().unwrap();
        if line.as_slice().is_empty() {
            // If we have exhausted this line, then ensuing call to
            // `next_token()` will call `advance_line()`
            self.line = None;
        }
        token
    }

    /// Add a new macro definition
    fn add_define(&mut self, macrodef: MacroDef) {
        let name = macrodef.name().to_owned();

        if let Some(original) = self.defines.get(&name) {
            if !original.equivalent(&macrodef) {
                self.tuctx.emit_message(
                    macrodef.location().clone(),
                    MessageKind::Phase4MacroRedefinitionDifferent {
                        name: name,
                        original: original.location().clone(),
                    },
                )
            }
        } else {
            self.defines.insert(name, Rc::new(macrodef));
        }
    }

    /// Remove a macro definition
    fn remove_define(&mut self, name: PPToken) {
        let macrodef = self.defines.remove(&name.value);
        if macrodef.is_none() {
            self.tuctx.emit_message(
                name.location,
                MessageKind::Phase4UndefineInvalidMacro { name: name.value },
            )
        }
    }

    /// Process directives until finding the first text line
    ///
    /// Return first token of the text line if one is found. Otherwise, return
    /// `None`, signaling EOF.
    fn advance_line(&mut self) -> Option<PPToken> {
        while let Some(directive) = self.directives.next() {
            match directive {
                Directive::Define(macrodef) => self.add_define(macrodef),
                Directive::Undefine(name) => self.remove_define(name),
                Directive::Text(tokens) => {
                    debug_assert!(!tokens.is_empty());
                    self.line = Some(tokens.into_iter());
                    return self.next_token();
                }
                Directive::IfSection { .. } => unreachable!(),
            }
        }
        None
    }

    /// Returns next token to be processed
    ///
    /// The priority is `self.rescan` (in reverse order), then `self.line`, then
    /// extracting a new value for `self.line` from `self.directives`.
    fn next_token(&mut self) -> Option<PPToken> {
        if self.rescan.is_empty() {
            if self.line.is_some() {
                Some(self.next_token_from_line())
            } else {
                self.advance_line()
            }
        } else {
            // should always return Option::Some
            self.rescan.pop()
        }
    }

    /// Parse arguments to a function-like macro
    fn parse_arguments(
        &mut self,
        func: &MacroFunction,
        open: &Location,
    ) -> HashMap<String, Vec<PPToken>> {
        trace!(
            "Expander::parse_arguments(func: {:?}, open: {:?})",
            func,
            open
        );

        // left-paren has already been consumed

        let mut depth = 0;
        let mut arguments = Vec::new();
        let mut current_arg = Vec::new();
        while let Some(token) = self.next_token() {
            trace!(
                "Expander::parse_arguments() token={} depth={} current_arg={:?} arguments={:?}",
                &token,
                depth,
                &current_arg,
                &arguments
            );

            if token.as_str() == "," && depth == 0 {
                if func.vararg && arguments.len() == func.params.len() {
                    // once we have parsed all the named arguments, we begin
                    // parsing the vararg arguments. For this, we provide one
                    // extra argument containing everything else (including
                    // the commas)
                    current_arg.push(token);
                } else {
                    arguments.push(std::mem::replace(&mut current_arg, Vec::new()));
                }
            } else if token.as_str() == "(" {
                current_arg.push(token);
                depth += 1;
            } else if token.as_str() == ")" && depth > 0 {
                current_arg.push(token);
                depth -= 1;
            } else if token.as_str() == ")" && depth == 0 {
                if !tokens_trim_whitespace(&current_arg).is_empty() || func.params.len() > 0 {
                    // an argument can be empty, but if the function expects 0
                    // arguments, don't push one
                    arguments.push(current_arg);
                }
                break;
            } else if token.kind == PPTokenKind::EndOfFile {
                self.tuctx.emit_message(
                    token.location,
                    MessageKind::Phase4UnclosedMacroInvocation {
                        name: func.name.clone(),
                        open: open.clone(),
                    },
                );
                return HashMap::new();
            } else {
                current_arg.push(token);
            }
        }

        // trim whitespace from beginning and ends of each argument
        for arg in &mut arguments {
            let trimmed = tokens_trim_whitespace(&arg).to_vec();
            *arg = trimmed;
        }

        let mut vararg = None;
        if func.vararg {
            // when expecting a vararg, there can only be one extra argument
            debug_assert!(arguments.len() <= func.params.len() + 1);

            if arguments.len() > func.params.len() {
                vararg = Some(arguments.pop().unwrap());
            } else {
                vararg = Some(Vec::new());
            }
        }

        trace!("Expander::parse_arguments() arguments={:?}", &arguments);
        trace!("Expander::parse_arguments() vararg={:?}", &vararg);

        if arguments.len() != func.params.len() {
            self.tuctx.emit_message(
                open.clone(),
                MessageKind::Phase4MacroArity {
                    name: func.name.clone(),
                    expected: func.params.len(),
                    found: arguments.len(),
                    vararg: func.vararg,
                },
            );
        }

        let mut parameters = HashMap::new();
        for (param, arg) in func.params.iter().zip(arguments.into_iter()) {
            parameters.insert(param.clone(), arg);
        }
        if let Some(vararg) = vararg {
            parameters.insert("__VA_ARGS__".to_owned(), vararg);
        }

        trace!("Expander::parse_arguments() parameters={:?}", &parameters);
        parameters
    }

    /// Perform macro replacement
    ///
    /// This includes function macro arguments as well as token stringifying and
    /// concatenation
    fn replace(
        &mut self,
        function: bool,
        mut input: IntoIter<PPToken>,
        parameters: HashMap<String, Vec<PPToken>>,
    ) -> Vec<PPToken> {
        trace!(
            "Expander::replace(function: {}, input: {:?}, parameters: {:?})",
            function,
            PPToken::to_strings(input.as_slice()),
            &parameters
        );

        let mut output = Vec::new();
        let mut skip_rhs_of_concat = false;
        while let Some(token) = input.next() {
            trace!("Expander::replace() loop token={}", &token);
            trace!(
                "Expander::replace() loop input={:?}",
                PPToken::to_strings(input.as_slice())
            );
            trace!(
                "Expander::replace() loop output={:?}",
                PPToken::to_strings(output.as_slice())
            );

            if let Some(replacement) = parameters.get(token.as_str()) {
                // We have a macro parameter. It can either be the left-hand
                // side of a `##` operator or it could be just a plain
                // substitution of the macro argument.

                // we consume whitespace here so we can detect `##`
                // however, if we do not find a `##`, then we need to output the whitespace
                let mut whitespace = Vec::new();
                while input.as_slice().get(0).map(|t| t.is_whitespace()) == Some(true) {
                    whitespace.push(input.next().unwrap());
                }

                if let Some("##") = input.as_slice().get(0).map(|t| t.as_str()) {
                    // We have to do some extra work here to correctly handle
                    // when one side is empty
                    if replacement.is_empty() {
                        input.next(); // consume `##`
                        line_skip_whitespace_until_newline(&mut input);

                        // index safe here because we reject macros that end in ##
                        let rhs = input.as_slice()[0].as_str();
                        if parameters.contains_key(rhs) {
                            // do not consume rhs macro name

                            // this parameter will get substituted but not
                            // expanded by the code that handles this boolean
                            skip_rhs_of_concat = true;
                        } else {
                            // push nothing to output
                            // so both the lhs and the `##` are ignored
                        }
                    } else {
                        // Leave `##` in input so it can be read in next
                        // iteration of loop. The concatenation will be handled
                        // by a different clause of the outer-most if statement
                        output.extend_from_slice(replacement);
                    }
                } else if skip_rhs_of_concat {
                    skip_rhs_of_concat = false;
                    output.extend_from_slice(replacement);
                } else {
                    // Plain parameter substitution, so take the parameter value and expand it
                    let expander =
                        Expander::from_tokens(self.tuctx, self.defines, replacement.clone());
                    output.append(&mut expander.expand());
                    output.append(&mut whitespace);
                }
            } else if token.as_str() == "#" && function {
                // we only stringize `#` tokens that occur within function macros

                // we reject macros where `#` is not followed by a parameter
                // (whitespace between is okay)
                line_skip_whitespace_until_newline(&mut input);
                let rhs = input.next().unwrap();

                output.push(stringize(&parameters[rhs.as_str()], token.location.clone()));
            } else if token.as_str() == "##" {
                // we reject macrodefs that begin or end with `##`, so there is
                // always another token on either side, however it may be a
                // whitespace token.

                let lhs = loop {
                    let token = output.pop().unwrap();
                    if !token.is_whitespace() {
                        break token;
                    }
                };
                line_skip_whitespace_until_newline(&mut input);
                let next = input.next().unwrap();
                trace!(
                    "Expander::replace() concatenation lhs={} next={}",
                    &lhs,
                    &next
                );

                // next may be a param name, in which case we should substitute
                // that (but not expand the replacement ), or it could be just a
                // plain token. If it is a parameter, it may substitute to many
                // tokens, in which case only the first will be considered the
                // rhs to be concatenated, and the others will be appended to
                // output after the result of concatenation.
                let rhs;
                let mut additional_output = None;
                if let Some(replacement) = parameters.get(next.as_str()) {
                    if replacement.is_empty() {
                        // parameter expanded to nothing, so nothing to concatenate with.
                        // thus we push lhs back on to output and discard the `##`
                        output.push(lhs);
                        continue;
                    } else {
                        // keep only first token of expansion as rhs, rest of
                        // expansion will be outputted directly afterwards
                        rhs = replacement[0].clone();
                        additional_output = Some(replacement[1..].to_vec());
                    }
                } else {
                    rhs = next;
                }

                let value = format!("{}{}", lhs.value, rhs.value);
                let (slice, kind) = lex_one_token(&value);

                if value.len() == slice.len() {
                    output.push(PPToken {
                        value,
                        kind,
                        location: token.location,
                    });
                    if let Some(mut additional_output) = additional_output {
                        output.append(&mut additional_output);
                    }
                } else {
                    // the token we lexed does not contain the entire
                    // concatenated string, thus indicating the concatenation
                    // did not result in a (single) valid token.
                    self.tuctx.emit_message(
                        token.location,
                        MessageKind::Phase4BadConcatenation {
                            lhs: lhs.value,
                            rhs: rhs.value,
                        },
                    );
                }
            } else {
                output.push(token);
            }
        }

        trace!(
            "Expander::replace() output={:?}",
            PPToken::to_strings(output.as_slice())
        );
        output
    }

    /// Inspect a single identifier and determine if it needs expanding
    fn expand_ident(&mut self, token: PPToken) {
        trace!("Expander::expand_ident(token: {})", &token);

        let macrodef = self.defines.get(&token.value);
        match macrodef.map(|d| &**d) {
            Some(MacroDef::Object(obj)) => {
                trace!("Expander::expand_ident() {:?}", &obj);

                let replacement = obj.replacement.clone().into_iter();
                let mut replaced =
                    self.replace(/* function-like? */ false, replacement, HashMap::new());

                disable_macro_recursion(&mut replaced, &token);
                self.rescan(replaced);
            }
            Some(MacroDef::Function(_)) => {
                // This nonsense with the Rc is a hack to work around borrow
                // checker. In particular, because we want to mutably borrow
                // self below to call `self.next_token()` and it would be
                // impossible to refactor the necessary fields into a separate
                // struct and use composition. An alternate solution would be to
                // perform this call to `self.next_token()` outside the match,
                // and simply push it back onto `self.rescan` when we don't need
                // it, but that probably has an even higher runtime cost.

                // TLDR: rust needs syntax to declare functions perform partial borrows
                let macrodef = Rc::clone(macrodef.unwrap());
                let func = macrodef.as_function();
                trace!("Expander::expand_ident() {:?}", &func);

                // For a function macro invocation, between the macro name and
                // its opening left paren, all whitespace should be ignored.
                // However, if we are not encountering a function macro
                // invocation, then output this whitespace.

                // We want to look ahead and see if the next non-whitespace
                // character is a left-paren. However, in case it is not, we
                // rescan those tokens so they can be correctly emitted
                let mut whitespace = Vec::new();
                let mut next = None;
                while let Some(token) = self.next_token() {
                    if token.is_whitespace() {
                        whitespace.push(token);
                    } else {
                        next = Some(token);
                        break;
                    }
                }

                trace!("Expander::expand_ident() next = {:?}", &next);
                if let Some(next) = next {
                    // next is guaranteed to be non-whitespace
                    if next.as_str() == "(" {
                        let arguments = self.parse_arguments(func, &next.location);
                        let replacement = func.replacement.clone().into_iter();
                        let mut replaced = self.replace(true, replacement, arguments);

                        disable_macro_recursion(&mut replaced, &token);
                        self.rescan(replaced);
                    } else if next.kind == PPTokenKind::Identifier {
                        // this ident is not being used as a function macro, so output it
                        self.output.push(token);
                        self.output.append(&mut whitespace);
                        // the next ident should be rescanned
                        self.rescan.push(next);
                        return;
                    } else {
                        // this ident is not being used as a function macro, so output it
                        self.output.push(token);
                        self.output.append(&mut whitespace);
                        // the next token also cannot be a macro, so just output it
                        self.output.push(next);
                        return;
                    }
                } else {
                    self.output.push(token);
                    self.output.append(&mut whitespace);
                }
            }
            _ => self.output.push(token),
        }
    }

    fn expand(mut self) -> Vec<PPToken> {
        trace!("Expander::expand()");
        while let Some(token) = self.next_token() {
            trace!("Expander::expand() token={}", &token);
            match token.kind {
                PPTokenKind::Identifier => {
                    self.expand_ident(token);
                }
                _ => {
                    self.output.push(token);
                }
            }
        }
        self.output
    }
}

fn stage_two(tuctx: &mut TUCtx, directives: Vec<Directive>) -> Vec<PPToken> {
    let mut defines = HashMap::new();
    let expander = Expander::from_directives(tuctx, &mut defines, directives);

    expander.expand()
}

pub fn preprocess(tuctx: &mut TUCtx, tokens: Vec<PPToken>) -> Vec<PPToken> {
    let lines = parse_lines(tokens);

    // Here we split processing into two stages. This allows a simple
    // implementation accommodating one of the more unintuitive uses of macros.
    // The primary motivation is handling mutli-line function macros that may
    // cross if-sections.
    // Consider this code:
    // ```
    // #define test(a) a
    // test(
    // #if 1
    // hello)
    // #endif
    // ```
    // Should expand to "hello"
    //
    // Thus, the first stage expands if sections, while the second stage expands
    // macros
    let directives = stage_one(tuctx, lines);
    stage_two(tuctx, directives)
}
