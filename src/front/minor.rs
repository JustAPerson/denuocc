// Copyright (C) 2020 Jason Priest
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

//! Minor phases: 1, 2, 5, 6

use std::convert::TryFrom;

use crate::message::MessageKind;
use crate::token::{CharToken, Location, PPToken, PPTokenKind};
use crate::tu::TUCtx;

/// Phase 1: Convert trigraphs
pub fn convert_trigraphs<'a>(tokens: Vec<CharToken>) -> Vec<CharToken> {
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

/// Phase 2: Splice together physical lines into logical lines
///
/// A line ending in `\` will be spliced together with the next line. Thus both
/// the back slash and newline characters will be removed. This allows multiline
/// comments and strings
pub fn splice_lines(tuctx: &mut TUCtx, input: Vec<CharToken>) -> Vec<CharToken> {
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

    output
}
