// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Passes for manipulating internal compiler state

use crate::declare_pass;
use crate::passes::Pass;
use crate::tu::TUCtx;
use crate::{ErrorKind, Result};

declare_pass!(
    /// Pretty-print [`TUCtx`][TUCtx]'s primary state to stderr
    state_print => pub struct StatePrint {}
);
impl Pass for StatePrint {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let state = tuctx.get_state()?;
        eprintln!("{}", state);
        Ok(())
    }
}

declare_pass!(
    /// Debug-print [`TUCtx`][TUCtx]'s primary state to stderr
    state_print_debug => pub struct StatePrintDebug {}
);
impl Pass for StatePrintDebug {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        let state = tuctx.get_state()?;
        eprintln!("{:#?}", state);
        Ok(())
    }
}

declare_pass!(
    /// Save the [`TUCtx`'s][tu] current primary state for later access by
    /// [`TranslationUnit::saved_states()`][tucs]
    ///
    /// [tu]: crate::tu::TUCtx
    /// [tucs]: crate::TranslationUnit::saved_states
    state_save => pub struct StateSave {
        pub name: String
    }
);
impl Pass for StateSave {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        tuctx.save_state(&self.name)
    }
}

declare_pass!(
    /// Pretty-print [`TUCtx`][TUCtx]'s primary state to file
    state_write => pub struct StateWrite {
        pub filename: String
    }
);
impl Pass for StateWrite {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        use std::io::Write;

        let state = tuctx.get_state()?;
        std::fs::File::open(&self.filename)
            .and_then(|mut f| write!(f, "{}", state))
            .map_err(|error| ErrorKind::OutputFileError {
                filename: self.filename.to_owned(),
                error,
            })?;

        Ok(())
    }
}

declare_pass!(
    /// Debug-print [`TUCtx`][TUCtx]'s primary state to file
    state_write_debug => pub struct StateWriteDebug {
        pub filename: String
    }
);
impl Pass for StateWriteDebug {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        use std::io::Write;
        let state = tuctx.get_state()?;
        std::fs::File::open(&self.filename)
            .and_then(|mut f| write!(f, "{:#?}", state))
            .map_err(|error| ErrorKind::OutputFileError {
                filename: self.filename.to_owned(),
                error,
            })?;

        Ok(())
    }
}

declare_pass!(
    /// Reads the specified input for this translation unit
    ///
    /// This must be the entry point of every collection of passes.
    state_read_input => pub struct StateReadInput {}
);
impl Pass for StateReadInput {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
        use crate::front::token::CharToken;
        let input = tuctx.original_input();
        let tokens = CharToken::from_input(input);
        tuctx.set_state(crate::tu::TUState::CharTokens(tokens));

        Ok(())
    }
}
