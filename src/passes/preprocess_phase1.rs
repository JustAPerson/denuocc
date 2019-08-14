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

//! Phase 1: Convert trigraphs

use crate::error::Result;
use crate::passes::helper::args_assert_count;
use crate::token::CharToken;
use crate::tu::{TUCtx, TUState};

pub fn preprocess_phase1_raw<'a>(tokens: Vec<CharToken>) -> Vec<CharToken> {
    static REPLACEMENTS: &[(char, char)] = &[
        ('=', '#'),
        (')', ']'),
        ('!', '|'),
        ('(', '['),
        ('\'', '^'),
        ('>', '}'),
        ('/', '\\'),
        ('<', '{'),
        ('-', '~'),
    ];

    let mut output = Vec::new();
    let mut iter = tokens.into_iter();

    while iter.as_slice().len() > 2 {
        // advance iter by one
        let first = iter.next().unwrap();

        // peek ahead two extra tokens (after next)
        let second = &iter.as_slice()[0];
        let third = &iter.as_slice()[1];

        if first.value == '?' && second.value == '?' {
            if let Some((_, to)) = REPLACEMENTS.iter().find(|(from, _)| *from == third.value) {
                output.push(CharToken {
                    value: *to,
                    loc: &third.loc - first.loc,
                });
                iter.next();
                iter.next();
                continue;
            }
        }

        // did not find any trigraphs
        output.push(first);
    }

    while let Some(token) = iter.next() {
        output.push(token);
    }

    output
}

/// Translate source character set (trigraphs etc)
pub fn preprocess_phase1<'a>(tuctx: &mut TUCtx<'a>, args: &[String]) -> Result<()> {
    args_assert_count("preprocess_phase1", args, 0)?;

    let tokens = tuctx.take_state()?.into_chartokens()?;
    let output = preprocess_phase1_raw(tokens);
    tuctx.set_state(TUState::CharTokens(output));

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::driver::Input;
    use std::rc::Rc;

    fn phase1_raw(content: &str) -> String {
        let input = Rc::new(Input {
            name: "<unit-test>".to_string(),
            content: content.to_string(),
            is_file: false,
        });
        let tokens = CharToken::from_input(&input);
        let output = preprocess_phase1_raw(tokens);
        CharToken::to_string(&output)
    }

    fn phase1_integration(input: &str) -> String {
        let mut driver = crate::driver::Driver::new();
        driver.add_input_str("<unit-test>", input);
        driver
            .parse_args_from_str(&[
                "--pass=state_read_input",
                "--pass=preprocess_phase1",
                "--pass=state_save(chartokens)",
            ])
            .unwrap();
        let tu = driver.run_one("<unit-test>").unwrap();
        let output = tu.saved_states("chartokens")[0]
            .clone()
            .into_chartokens()
            .unwrap();
        CharToken::to_string(&output)
    }

    fn test_both_phase1(input: &str, expected: &str) {
        assert_eq!(phase1_raw(input), expected);
        assert_eq!(phase1_integration(input), expected);
    }

    #[test]
    fn test_phase1() {
        test_both_phase1("a", "a");
        test_both_phase1("ab", "ab");
        test_both_phase1("abc", "abc");

        test_both_phase1("?", "?");
        test_both_phase1("??", "??");
        test_both_phase1("???", "???");

        test_both_phase1("??(", "[");
        test_both_phase1("??)", "]");
        test_both_phase1("??<", "{");
        test_both_phase1("??>", "}");
        test_both_phase1("??=", "#");
        test_both_phase1("??!", "|");
        test_both_phase1("??'", "^");
        test_both_phase1("??-", "~");
        test_both_phase1("??/", "\\");

        test_both_phase1("???=", "?#");
        test_both_phase1("????=", "??#");

        test_both_phase1(
            "??=define arraycheck(a, b) a??(b??) ??!??! b??(a??)",
            "#define arraycheck(a, b) a[b] || b[a]",
        );
        test_both_phase1(r#"printf("Eh???/n");"#, r#"printf("Eh?\n");"#);
    }
}
