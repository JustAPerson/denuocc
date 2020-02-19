// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Define pass functions for each phase of compilation

use crate::error::Result;
use crate::front::lexer::lex;
use crate::front::minor::{concatenate, convert_trigraphs, splice_lines, unescape};
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

/// Calls [`front::minor::concatenate`](concatenate)
pub fn phase6(tuctx: &mut TUCtx, args: &[String]) -> Result<()> {
    args_assert_count("phase6", args, 0)?;

    let tokens = tuctx.take_state()?.into_pptokens()?;
    let output = concatenate(tuctx, tokens);
    tuctx.set_state(TUState::PPTokens(output));

    Ok(())
}
