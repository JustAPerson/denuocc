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

//! Phase 2: Splice together physical lines into logical lines
//!
//! A line ending in `\` will be spliced together with the next line. Thus both
//! the back slash and newline characters will be removed. This allows multiline
//! comments and strings

use crate::error::Result;
use crate::message::MessageKind;
use crate::passes::helper::args_assert_count;
use crate::tu::{TUCtx, TUState};

/// Splice together lines ending in backslash
pub fn preprocess_phase2(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("preprocess_phase2", args, 0)?;

    let input = tuctx.take_state()?.into_chartokens()?;

    let mut output = Vec::new();
    let mut iter = input.into_iter();

    while iter.as_slice().len() > 1 {
        let first = iter.next().unwrap();
        let second = &iter.as_slice()[0];

        if first.value == '\\' && second.value == '\n' {
            let loc = &second.loc - second.loc.clone();
            iter.next(); // consume second

            // do not emit either to output, in effect splicing physical lines
            // into one logical line

            // are these the last two characters of input?
            if iter.as_slice().len() == 0 {
                tuctx.emit_message(loc, MessageKind::Phase1FileEndingWithBackslash);
            }
        } else {
            output.push(first);
        }
    }

    if let Some(last) = iter.next() {
        if last.value == '\\' {
            tuctx.emit_message(last.loc, MessageKind::Phase1FileEndingWithBackslash);
        } else {
            output.push(last);
        }
    }
    assert!(iter.next().is_none());

    tuctx.set_state(TUState::CharTokens(output));

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::token::CharToken;

    fn test_phase2_case(input: &str, expected: &str, num_messages: usize) {
        let mut driver = crate::driver::Driver::new();
        driver.add_input_str("<unit-test>", input);
        driver
            .parse_args_from_str(&[
                "--pass=state_read_input",
                "--pass=preprocess_phase1",
                "--pass=preprocess_phase2",
                "--pass=state_save(chartokens)",
            ])
            .unwrap();
        let mut tu = driver.run_one("<unit-test>").unwrap();
        let output = tu.saved_states("chartokens")[0]
            .clone()
            .into_chartokens()
            .unwrap();
        let result = CharToken::to_string(&output);
        assert_eq!(result, expected);
        assert_eq!(tu.take_messages().len(), num_messages);
    }

    #[test]
    fn test_phase2() {
        test_phase2_case("a\nb", "a\nb", 0);
        test_phase2_case("a\\\nb", "ab", 0);
        test_phase2_case("a\\\\\nb", "a\\b", 0);
        test_phase2_case("a\\\n\\\nb", "ab", 0);

        // the standard forbids and input ending in either of the following
        // but we simply ignore and generate a warning
        test_phase2_case("\\", "", 1);
        test_phase2_case("\\\n", "", 1);

        // test that trigraphs can also splice lines
        test_phase2_case("a??/\nb", "ab", 0);
    }
}
