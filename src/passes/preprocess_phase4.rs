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

/// Phase 4: Execute preprocessor directives
use std::collections::HashMap;
use std::vec::IntoIter;

use crate::error::Result;
use crate::message::MessageKind::{
    ExpectedFound, Phase4DefineOperator, Phase4ExpectedPPToken, Phase4InvalidDirective,
    Phase4MacroArity, Phase4MacroArityVararg, Phase4MacroType, Phase4UnexpectedDirective,
    Phase4ExpectedNewline,
};
use crate::passes::helper::args_assert_count;
use crate::token::{PPToken, PPTokenKind};
use crate::tu::{TUCtx, TUState};

#[derive(Clone, Debug)]
enum MacroDef {
    Object {
        name: String,
        replacements: Vec<PPToken>,
    },
    Function {
        name: String,
        replacements: Vec<PPToken>,
        params: Vec<String>,
        vararg: bool,
    },
}

struct PPTokenStream<'a, 'b> {
    tuctx: &'a mut TUCtx<'b>,
    // tokens: Vec<PPToken>,
    tokens: IntoIter<PPToken>,
    macrodefs: HashMap<String, MacroDef>,
    output: Vec<PPToken>,
}

impl<'a, 'b> PPTokenStream<'a, 'b> {
    fn new(tuctx: &'a mut TUCtx<'b>, tokens: Vec<PPToken>) -> Self {
        Self {
            tuctx,
            tokens: tokens.into_iter(),
            macrodefs: HashMap::new(),
            output: Vec::new(),
        }
    }

    fn peek(&self) -> &PPToken {
        &self.tokens.as_slice()[0]
    }

    fn skip_token(&mut self) -> PPToken {
        self.tokens.next().unwrap()
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_whitespace() {
            self.skip_token();
        }
    }

    fn skip_whitespace_until_newline(&mut self) {
        while self.peek().is_whitespace() && self.peek().as_str() != "\n" {
            self.skip_token();
        }
    }

    fn skip_until_newline(&mut self) -> Vec<PPToken> {
        let mut output = Vec::new();
        while self.peek().as_str() != "\n" && self.peek().kind != PPTokenKind::EndOfFile {
            output.push(self.skip_token());
        }
        output
    }

    fn get_defined_result(&mut self, token: &PPToken) -> PPToken {
        PPToken {
            kind: PPTokenKind::PPNumber,
            value: if self.macrodefs.contains_key(token.as_str()) {
                "1"
            } else {
                "0"
            }
            .to_owned(),
            location: token.location.clone(),
        }
    }

    /// Expand the `defined` operator in `#if` and `#elif` directives
    fn expand_defined_operator(&mut self, input: Vec<PPToken>) -> Vec<PPToken> {
        let mut output = Vec::new();
        let mut iter = input.into_iter();

        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        enum State {
            Begin,       // seen nothing
            Defined,     // seen `defined` expect left-paren or ident
            LParen,      // seen `defined (` expect ident
            LParenIdent, // seen `defined ( <ident>` expect right-paren
        };

        let mut state = State::Begin;

        while iter.as_slice().len() > 0 {
            let token = iter.next().unwrap();

            if token.is_whitespace() {
                continue;
            }

            match (state, token.kind, token.as_str()) {
                // if we encounter a "defined" identifier, we begin parsing a
                // defined operator otherwise we just push the token to the
                // output so it can be handled by later steps
                (State::Begin, PPTokenKind::Identifier, "defined") => {
                    state = State::Defined;
                }
                (State::Begin, ..) => {
                    output.push(token);
                }

                // if we have seen a "defined" identifier, we expect to either
                // see another identifier or a left-paren
                (State::Defined, PPTokenKind::Identifier, ..) => {
                    // if we find an identifier, then we have finished the plain
                    // `defined <ident>` pattern
                    state = State::Begin;
                    output.push(self.get_defined_result(&token));
                }
                (State::Defined, PPTokenKind::Punctuator, "(") => {
                    state = State::LParen;
                }
                (State::Defined, ..) => {
                    state = State::Begin;
                    self.tuctx
                        .emit_message(token.location, Phase4DefineOperator);
                }

                // after a left-paren, we expect an identifier
                (State::LParen, PPTokenKind::Identifier, ..) => {
                    // note unlike above, this is not the end of the parsing. we
                    // expect a right-paren as well
                    state = State::LParenIdent;
                    output.push(self.get_defined_result(&token));
                }
                (State::LParen, ..) => {
                    // upon an error we just reset state
                    state = State::Begin;
                    self.tuctx
                        .emit_message(token.location, Phase4DefineOperator);
                }

                // After `defined ( <ident>` we expect a closing right-paren
                (State::LParenIdent, PPTokenKind::Punctuator, ")") => {
                    state = State::Begin;
                }
                (State::LParenIdent, ..) => {
                    state = State::Begin;
                    self.tuctx
                        .emit_message(token.location, Phase4DefineOperator);
                }
            }
        }

        output
    }

    fn match_macro_function(
        &mut self,
        num_params: usize,
        vararg: bool,
        iter: &mut std::vec::IntoIter<PPToken>,
    ) -> (Vec<Vec<PPToken>>, Vec<PPToken>) {
        let mut args: Vec<Vec<PPToken>> = Vec::with_capacity(num_params);
        let mut vararg_arg: Vec<PPToken> = Vec::new();

        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
        enum State {
            Arg,
            Vararg,
            Finished,
        }

        let mut state = if num_params > 0 {
            State::Arg
        } else {
            State::Vararg
        };
        let mut arg = Vec::new();

        let mut last_token = None;
        while let Some(token) = iter.next() {
            match (state, token.as_str()) {
                (State::Arg, ",") => {
                    let arg_done = std::mem::replace(&mut arg, Vec::new());
                    args.push(arg_done);
                    if vararg && args.len() >= num_params {
                        state = State::Vararg;
                    }
                }
                (State::Arg, ")") => {
                    state = State::Finished;
                    let arg_done = std::mem::replace(&mut arg, Vec::new());
                    args.push(arg_done);
                    break;
                }
                (State::Arg, _) => {
                    arg.push(token.clone());
                }

                (State::Vararg, ")") => {
                    state = State::Finished;
                    break;
                }
                (State::Vararg, _) => {
                    vararg_arg.push(token.clone());
                }

                // should have broken out of loop already
                (State::Finished, _) => unreachable!(),
            }
            last_token = Some(token);
        }
        if state != State::Finished {
            unimplemented!("what do i call this error? {:#?}", last_token) // TODO NYI
        }

        (args, vararg_arg)
    }

    fn expand_macro_function(
        &mut self,
        name: String,
        iter: &mut std::vec::IntoIter<PPToken>,
    ) -> Vec<PPToken> {
        let lparen = iter.next().unwrap();
        debug_assert_eq!(lparen.as_str(), "(");

        let (vararg, params, replacements) = {
            if let MacroDef::Function {
                vararg,
                ref params,
                ref replacements,
                ..
            } = self.macrodefs[&name]
            {
                (vararg, params.clone(), replacements.clone())
            } else {
                // this function is only called for MacroDef::Functions
                unreachable!();
            }
        };

        let (mut args, vararg_arg) = self.match_macro_function(params.len(), vararg, iter);
        args = args
            .into_iter()
            .map(|arg| self.expand_macros(arg))
            .collect();

        dbg!(vararg);
        dbg!(&args);
        if vararg && args.len() < params.len() {
            self.tuctx.emit_message(
                lparen.location.clone(),
                Phase4MacroArityVararg {
                    name: name.clone(),
                    expected: params.len(),
                    found: args.len(),
                },
            );
        } else if !vararg
            && (args.len() + (if vararg_arg.is_empty() { 0 } else { 1 })) != params.len()
        {
            self.tuctx.emit_message(
                lparen.location.clone(),
                Phase4MacroArity {
                    name: name.clone(),
                    expected: params.len(),
                    found: args.len(),
                },
            );
        }

        let mut old: HashMap<String, MacroDef> = HashMap::new();
        for param in &params {
            if self.macrodefs.contains_key(param) {
                old.insert(param.to_owned(), self.macrodefs[param].clone());
            }
        }

        for (name, rep) in params.iter().zip(args) {
            self.macrodefs_insert_object(name, rep);
        }
        self.macrodefs_insert_object("__VA_ARGS__", vararg_arg);

        let macrodef = self.macrodefs.remove(&name).unwrap();
        let output = self.expand_macros(replacements);
        self.macrodefs.insert(name.clone(), macrodef);

        self.macrodefs.remove("__VA_ARGS__");
        for name in &params {
            self.macrodefs.remove(name);
        }
        for (name, def) in old {
            self.macrodefs.insert(name, def);
        }

        output
    }

    fn expand_macros(&mut self, input: Vec<PPToken>) -> Vec<PPToken> {
        // TODO optimize remove some of these output vecs, just use self.output
        // do after more tests in place
        let mut output = Vec::new();
        let mut iter = input.into_iter();

        while let Some(token) = iter.next() {
            if token.kind != PPTokenKind::Identifier || !self.macrodefs.contains_key(&token.value) {
                output.push(token);
                continue;
            }
            let macrodef = &self.macrodefs[&token.value];

            match macrodef {
                MacroDef::Object { name, .. }
                    if iter.as_slice().get(0).map(|t| t.as_str()) == Some("(") =>
                {
                    self.tuctx.emit_message(
                        token.location,
                        Phase4MacroType {
                            name: name.clone(),
                            defined: "object",
                            used: "function",
                        },
                    );
                }
                MacroDef::Function { name, .. }
                    if iter.as_slice().get(0).map(|t| t.as_str()) != Some("(") =>
                {
                    self.tuctx.emit_message(
                        token.location,
                        Phase4MacroType {
                            name: name.clone(),
                            defined: "function",
                            used: "object",
                        },
                    );
                }

                MacroDef::Object { replacements, .. } => {
                    // let replacements = self.expand_macros(replacements.clone());
                    let replacements = replacements.clone();
                    output.append(&mut self.expand_macros(replacements));
                }
                MacroDef::Function { name, .. } => {
                    let name = name.clone();
                    output.append(&mut self.expand_macro_function(name, &mut iter));

                    // match_macro_function(), called by expand_macro_function(),
                    // will take care of the closing paren for us
                }
            }
        }

        output
    }

    fn macrodefs_insert_object(&mut self, name: &str, replacements: Vec<PPToken>) {
        self.macrodefs.insert(
            name.to_owned(),
            MacroDef::Object {
                name: name.to_owned(),
                replacements,
            },
        );
    }

    /// Execute an `#if` directive
    fn execute_if_section(&mut self) {
        debug_assert!(["if", "ifdef", "ifndef"].contains(&self.peek().as_str()));
        let directive = self.skip_token();

        let value = match directive.as_str() {
            "if" => {
                let mut line = self.skip_until_newline();
                line = self.expand_defined_operator(line);
                line = self.expand_macros(line);
                line = line.into_iter().skip_while(|t| t.is_whitespace()).collect();
                // TODO parse
            }
            "ifdef" => {}
            "ifndef" => {}
            _ => unreachable!(),
        };
    }

    /// Execute a `#define` directive
    fn execute_define(&mut self) {
        self.skip_token(); // skip `define`
        self.skip_whitespace();

        if self.peek().kind != PPTokenKind::Identifier {
            self.tuctx.emit_message(
                self.peek().location.clone(),
                Phase4ExpectedPPToken {
                    expected: PPTokenKind::Identifier,
                    found: self.peek().kind,
                },
            );

            self.skip_until_newline();
            return;
        }

        let name = self.skip_token().value;
        if self.macrodefs.contains_key(&name) {
            // TODO remove this and permit exact macro redefinitions and warn on overwrites
            unimplemented!();
        }
        self.skip_whitespace();

        if self.peek().as_str() == "(" {
            self.skip_token();

            let mut vararg = false;
            let mut params = Vec::new();

            #[derive(Copy, Clone, Debug, PartialEq, Eq)]
            enum State {
                LParenOrComma,
                Ident,
                Vararg,
            }
            let mut state = State::LParenOrComma;

            while self.peek().kind != PPTokenKind::EndOfFile {
                let token = self.skip_token();
                if token.is_whitespace() {
                    continue;
                }

                match (state, token.kind, token.as_str()) {
                    (_, _, ")") => break,

                    (State::LParenOrComma, PPTokenKind::Identifier, ..) => {
                        state = State::Ident;
                        params.push(token.value); // FIXME ?can this clone be avoided?
                    }
                    (State::LParenOrComma, _, "...") => {
                        state = State::Vararg;
                        vararg = true;
                    }
                    (State::LParenOrComma, ..) => self.tuctx.emit_message(
                        // TODO FIXME error should skip rest of directive
                        token.location,
                        ExpectedFound {
                            expected: "identifier, `)`, or `...`".to_owned(),
                            found: format!("`{}`", token.value),
                        },
                    ),

                    (State::Ident, _, ",") => {
                        state = State::LParenOrComma;
                    }
                    (State::Ident, ..) => {
                        // TODO FIXME error should skip rest of directive
                        state = State::LParenOrComma;
                        self.tuctx.emit_message(
                            token.location,
                            ExpectedFound {
                                expected: "comma".to_owned(),
                                found: format!("`{}`", token.value),
                            },
                        )
                    }

                    // closing paren handled by first pattern in match
                    (State::Vararg, ..) => self.tuctx.emit_message(
                        token.location,
                        ExpectedFound {
                            expected: "identifier, `)`, or `...`".to_owned(),
                            found: format!("`{}`", token.value),
                        },
                    ),
                }
            }

            let replacements = self.skip_until_newline();
            self.macrodefs.insert(
                name.clone(),
                MacroDef::Function {
                    name,
                    replacements,
                    params,
                    vararg,
                },
            );
        } else {
            let replacements = self.skip_until_newline();
            self.macrodefs
                .insert(name.clone(), MacroDef::Object { name, replacements });
        }
    }

    fn execute_undef(&mut self) {
        self.skip_token(); // skip `define`
        self.skip_whitespace_until_newline();

        if self.peek().kind != PPTokenKind::Identifier {
            // invalid directive
            self.tuctx.emit_message(self.peek().location.clone() , Phase4ExpectedPPToken {
                expected: PPTokenKind::Identifier,
                found: self.peek().kind,
            });

            // ignore directive
            self.skip_until_newline();
            return;
        }

        let ident = self.skip_token();
        self.skip_whitespace_until_newline();

        if self.peek().as_str() != "\n" {
            self.tuctx.emit_message(self.peek().location.clone() , Phase4ExpectedNewline {
                found: self.peek().kind,
            });

            // ignore extra tokens on this line
            self.skip_until_newline();
        }

        if self.macrodefs.contains_key(&ident.value) {
            self.macrodefs.remove(&ident.value);
        }
    }

    fn execute_group(&mut self) {
        loop {
            self.skip_whitespace();

            if self.peek().kind == PPTokenKind::Punctuator && self.peek().as_str() == "#" {
                self.skip_token();
                self.skip_whitespace();
                match self.peek().as_str() {
                    "if" | "ifdef" | "ifndef" => self.execute_if_section(),
                    "elif" | "else" | "endif" => self.tuctx.emit_message(
                        self.peek().location.clone(),
                        Phase4UnexpectedDirective {
                            directive: self.peek().as_str().to_owned(),
                        },
                    ),
                    "define" => self.execute_define(),
                    "undef" => self.execute_undef(),
                    _ => self.tuctx.emit_message(
                        self.peek().location.clone(),
                        Phase4InvalidDirective {
                            directive: self.peek().as_str().to_owned(),
                        },
                    ),
                }
            } else if self.peek().kind != PPTokenKind::EndOfFile {
                let mut line = self.skip_until_newline();
                line = self.expand_macros(line);
                self.output.append(&mut line);
            } else {
                // EndOfFile
                break;
            }
        }
    }
}

pub fn preprocess_phase4(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("preprocess_phase3", args, 0)?;

    let tokens = tuctx.take_state()?.into_pptokens()?;
    let output = {
        let mut stream = PPTokenStream::new(tuctx, tokens);
        stream.execute_group();
        stream.output
    };
    tuctx.set_state(TUState::PPTokens(output));

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::driver::Driver;
    use crate::message::Message;

    #[test]
    fn test_expand_defined_operator() {
        fn case(input: &str) -> (Vec<PPToken>, Vec<Message>) {
            let mut driver = Driver::new();
            driver.add_input_str("<unit-test>", input);
            driver
                .parse_args_from_str(&[
                    "--pass=state_read_input",
                    "--pass=preprocess_phase3",
                    // notably not running phase4
                    "--pass=state_save(pptokens)",
                ])
                .unwrap();
            let mut tu = driver.run_one("<unit-test>").unwrap();
            let tokens = tu.saved_states("pptokens")[0]
                .clone()
                .into_pptokens()
                .unwrap();
            let mut stream = PPTokenStream::new(&mut tu, tokens.clone());
            let output = stream.expand_defined_operator(tokens);
            let messages = tu.take_messages();

            (output, messages)
        }

        let (output, messages) = case("defined a");
        assert_eq!(output[0].as_str(), "0");
        assert_eq!(messages.len(), 0);

        let (output, messages) = case("defined(a)");
        assert_eq!(output[0].as_str(), "0");
        assert_eq!(messages.len(), 0);

        let (output, messages) = case("defined ( a )");
        assert_eq!(output[0].as_str(), "0");
        assert_eq!(messages.len(), 0);

        let (_, messages) = case("defined + a )");
        assert_eq!(messages.len(), 1);

        let (_, messages) = case("defined ( 5 )");
        assert_eq!(messages.len(), 1);

        let (_, messages) = case("defined ( a x");
        assert_eq!(messages.len(), 1);

        // testing whether the value is 0 or 1 will occur in another test
    }
}
