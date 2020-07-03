// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Passes for the C front end (preprocessing, parsing, type checking)

use std::rc::Rc;

use crate::core::Result;
use crate::declare_pass;
use crate::front::c::lexer::lex;
use crate::front::c::minor::{concatenate, convert_trigraphs, splice_lines, unescape};
use crate::front::c::preprocessor::preprocess;
use crate::passes::Pass;
use crate::tu::{TUCtx, TUState};

declare_pass!(
    /// Calls [`front::minor::convert_trigraphs`](convert_trigraphs)
    phase1 => pub struct Phase1 {}
);
impl Pass for Phase1 {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let tokens = tuctx.take_state()?.into_chartokens()?;
        let output = convert_trigraphs(tokens);
        tuctx.set_state(TUState::CharTokens(output));

        Ok(())
    }
}

declare_pass!(
    /// Calls [`front::minor::splice_lines`](splice_lines)
    phase2 => pub struct Phase2 {}
);
impl Pass for Phase2 {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let tokens = tuctx.take_state()?.into_chartokens()?;
        let output = splice_lines(tuctx, tokens);
        tuctx.set_state(TUState::CharTokens(output));

        Ok(())
    }
}

declare_pass!(
    /// Calls [`front::lexer::lex`](lex)
    phase3 => pub struct Phase3 {}
);
impl Pass for Phase3 {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let tu_input = Rc::clone(tuctx.original_input());
        let tokens = tuctx.take_state()?.into_chartokens()?;
        let output = lex(tuctx, tokens, tu_input);
        tuctx.set_state(TUState::PPTokens(output));

        Ok(())
    }
}

declare_pass!(
    /// Calls [`front::preprocessor::preprocess`](preprocess)
    phase4 => pub struct Phase4 {}
);
impl Pass for Phase4 {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let tokens = tuctx.take_state()?.into_pptokens()?;
        let output = preprocess(tuctx, tokens);
        tuctx.set_state(TUState::PPTokens(output));

        Ok(())
    }
}

declare_pass!(
    /// Calls [`front::minor::unescape`](unescape)
    phase5 => pub struct Phase5 {}
);
impl Pass for Phase5 {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let mut tokens = tuctx.take_state()?.into_pptokens()?;
        unescape(tuctx, &mut tokens);
        tuctx.set_state(TUState::PPTokens(tokens));

        Ok(())
    }
}

declare_pass! {
    /// Calls [`front::minor::concatenate`](concatenate)
    phase6 => pub struct Phase6 {}
}
impl Pass for Phase6 {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let tokens = tuctx.take_state()?.into_pptokens()?;
        let output = concatenate(tuctx, tokens);
        tuctx.set_state(TUState::PPTokens(output));

        Ok(())
    }
}
