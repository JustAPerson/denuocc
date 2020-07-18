// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Translation Unit

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use crate::core::{ErrorKind, Result};
use crate::front::c::input::Input;
use crate::front::c::message::Message;
use crate::front::c::tuctx::{TUCtx, TUState};
use crate::session::Session;

/// Permanent data for a translation unit
#[derive(Clone, Debug)]
pub struct TranslationUnit {
    pub(super) session: Rc<Session>,
    pub(super) input: Rc<Input>,
    pub(super) messages: Vec<Message>,
    pub(super) saved_states: HashMap<String, Vec<TUState>>,
    pub(super) success: bool,
}

impl TranslationUnit {
    /// Entry point for creating a `TranslationUnit`
    pub fn builder(session: &Rc<Session>) -> TranslationUnitBuilder {
        TranslationUnitBuilder {
            session: Rc::clone(session),
            input: None,
        }
    }

    /// Original source code
    pub fn input(&self) -> &Rc<Input> {
        &self.input
    }

    /// Messages generated during processing
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// States saved by the [`state_save`][ss] pass
    ///
    /// [ss]: crate::passes::internal::StateSave
    pub fn saved_states(&self, name: &str) -> &[TUState] {
        &self.saved_states[name]
    }

    /// Whether translation succeeded
    pub fn success(&self) -> bool {
        self.success
    }

    pub fn run(&mut self) -> Result<()> {
        let mut ctx = TUCtx::from_tu(self);
        self.success = ctx.run()?;
        Ok(())
    }
}

pub struct TranslationUnitBuilder {
    session: Rc<Session>,
    input: Option<Rc<Input>>,
}

impl TranslationUnitBuilder {
    pub fn build(self) -> TranslationUnit {
        TranslationUnit {
            session: self.session,
            input: self.input.expect("must provide an input"),
            messages: Vec::new(),
            saved_states: HashMap::new(),

            success: false,
        }
    }

    fn assert_no_input(&self) {
        assert!(
            self.input.is_none(),
            "cannot specify multiple sources/inputs"
        );
    }

    pub fn source_file(mut self, path: &Path) -> Result<Self> {
        self.assert_no_input();

        let name = path.to_string_lossy().into_owned();
        let content = std::fs::read_to_string(path).map_err(|e| ErrorKind::InputFileError {
            filename: name.to_owned(),
            error: e,
        })?;

        // make sure path we store is rooted
        let mut pathbuf = std::env::current_dir().unwrap();
        pathbuf.push(path);
        let input = Input::new(name.clone(), content, Some(pathbuf));
        self.input = Some(Rc::new(input));

        Ok(self)
    }

    pub fn source_string(mut self, alias: impl Into<String>, content: impl Into<String>) -> Self {
        let alias = alias.into();
        let content = content.into();
        assert!(
            alias.starts_with("<") && alias.ends_with(">"),
            "alias must be enclosed in <> brackets"
        );
        self.assert_no_input();
        self.input = Some(Rc::new(Input::new(alias, content, None)));
        self
    }
}
