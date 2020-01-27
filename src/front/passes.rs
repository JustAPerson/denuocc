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

//! Define pass functions for each phase of compilation

use crate::error::Result;
use crate::front::lexer::lex;
use crate::front::minor::{convert_trigraphs, splice_lines, unescape};
use crate::front::preprocessor::preprocess;
use crate::passes::helper::args_assert_count;
use crate::tu::{TUCtx, TUState};

/// Calls [`front::minor::convert_trigraphs`](convert_trigraphs)
pub fn phase1(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("phase1", args, 0)?;

    let tokens = tuctx.take_state()?.into_chartokens()?;
    let output = convert_trigraphs(tokens);
    tuctx.set_state(TUState::CharTokens(output));

    Ok(())
}

/// Calls [`front::minor::splice_lines`](splice_lines)
pub fn phase2(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("phase2", args, 0)?;

    let tokens = tuctx.take_state()?.into_chartokens()?;
    let output = splice_lines(tuctx, tokens);
    tuctx.set_state(TUState::CharTokens(output));

    Ok(())
}

/// Calls [`front::lexer::lex`](lex)
pub fn phase3(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("phase3", args, 0)?;

    let tokens = tuctx.take_state()?.into_chartokens()?;
    let output = lex(tuctx, tokens);
    tuctx.set_state(TUState::PPTokens(output));

    Ok(())
}

/// Calls [`front::preprocessor::preprocess`](preprocess)
pub fn phase4(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("phase4", args, 0)?;

    let tokens = tuctx.take_state()?.into_pptokens()?;
    let output = preprocess(tuctx, tokens);
    tuctx.set_state(TUState::PPTokens(output));

    Ok(())
}

/// Calls [`front::minor::unescape`](unescape)
pub fn phase5(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("phase5", args, 0)?;

    let mut tokens = tuctx.take_state()?.into_pptokens()?;
    unescape(tuctx, &mut tokens);
    tuctx.set_state(TUState::PPTokens(tokens));

    Ok(())
}
