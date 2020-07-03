// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Translation Unit

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use log::debug;

use crate::core::{ErrorKind, Result};
use crate::front::input::{IncludedFrom, Input};
use crate::front::location::Location;
use crate::front::message::{Message, MessageKind};
use crate::front::token::{CharToken, PPToken};
use crate::session::Session;

/// Permanent data for a translation unit
#[derive(Clone, Debug)]
pub struct TranslationUnit {
    session: Rc<Session>,
    input: Rc<Input>,
    messages: Vec<Message>,
    saved_states: HashMap<String, Vec<TUState>>,
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

    pub fn run(&mut self) -> Result<()> {
        let mut ctx = TUCtx::from_tu(self);
        ctx.run()?;
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
/// Translation Unit State
///
/// This is the primary intermediate state that is shared between passes.
/// Auxiliary state may be kept in [`TUCtx`].
///
/// [`TUCtx`]: ./struct.TUCtx.html
#[derive(Clone, Debug)]
pub enum TUState {
    CharTokens(Vec<CharToken>),
    PPTokens(Vec<PPToken>),
}

macro_rules! into_methods {
    ($(($into_method:ident, $as_method:ident, $variant:ident, $returns:ty)),+) => ($(
        pub fn $into_method(self) -> Result<$returns> {
            match self {
                TUState::$variant(val) => Ok(val),
                other => Err(ErrorKind::TUStateTypeError {
                    current_type: other.kind(),
                    expected_type: stringify!($variant),
                }.into()),
            }
        }

        pub fn $as_method(&self) -> Result<&$returns> {
            match self {
                TUState::$variant(val) => Ok(&val),
                other => Err(ErrorKind::TUStateTypeError {
                    current_type: other.kind(),
                    expected_type: stringify!($variant),
                }.into()),
            }
        }
    )+)
}

impl TUState {
    pub fn kind(&self) -> &'static str {
        use TUState::*;
        match self {
            CharTokens(..) => "CharTokens",
            PPTokens(..) => "PPTokens",
        }
    }

    into_methods! {
        (into_chartokens, as_chartokens, CharTokens, Vec<CharToken>),
        (into_pptokens, as_pptokens, PPTokens, Vec<PPToken>)
    }
}

impl std::fmt::Display for TUState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use TUState::*;
        match self {
            CharTokens(tokens) => write!(f, "{}", CharToken::to_string(tokens)),
            PPTokens(tokens) => write!(f, "{}", PPToken::to_string(tokens)),
        }
    }
}

/// Intermediate data kept while processing this translation unit
#[derive(Debug)]
pub struct TUCtx<'a> {
    tu: &'a mut TranslationUnit,
    inputs: Vec<Rc<Input>>,
    state: Option<TUState>,
}

impl<'a> TUCtx<'a> {
    pub fn from_tu(tu: &'a mut TranslationUnit) -> TUCtx<'a> {
        let mut inputs = Vec::new();
        inputs.push(Rc::clone(&tu.input));

        TUCtx {
            tu,
            inputs,
            state: None,
        }
    }

    /// Returns the corresponding input for this unit
    pub fn original_input(&self) -> &Rc<Input> {
        &self.inputs[0]
    }

    /// Saves the current state, associating it with the given name
    ///
    /// Implicitly used in the [`state_save`][ss] pass.
    ///
    /// [ss]: crate::passes::internal::StateSave
    pub fn save_state(&mut self, name: &str) -> Result<()> {
        let state = self.get_state()?.clone();
        let entry = self
            .tu
            .saved_states
            .entry(name.to_owned())
            .or_insert_with(Vec::new);
        entry.push(state);
        Ok(())
    }

    /// Takes the existing primary state out of this object
    pub fn take_state(&mut self) -> Result<TUState> {
        self.state.take().ok_or(ErrorKind::TUStateAbsent.into())
    }

    /// Get a reference to the primary internal state
    pub fn get_state(&self) -> Result<&TUState> {
        self.state.as_ref().ok_or(ErrorKind::TUStateAbsent.into())
    }

    /// Get a mutable reference to the primary internal state
    pub fn get_mut_state(&mut self) -> Result<&mut TUState> {
        self.state.as_mut().ok_or(ErrorKind::TUStateAbsent.into())
    }

    /// Overwrite the primary internal state
    pub fn set_state(&mut self, state: TUState) {
        self.state = Some(state);
    }

    /// Emit an error to this translation unit's list
    pub fn emit_message(&mut self, location: impl Into<Location>, kind: MessageKind) {
        self.tu.messages.push(Message {
            location: location.into(),
            kind,
        });
    }

    /// Search for a file and include it in this translation unit's context
    pub fn add_include(
        &mut self,
        desired_file: &str,
        system: bool,
        included_from: IncludedFrom,
    ) -> Option<&Rc<Input>> {
        let including_file = included_from
            .input
            .path
            .as_ref()
            .map(|p| p.as_path())
            .clone();
        let input = self
            .tu
            .session
            .search_for_include(desired_file, including_file, system);

        if let Some(mut input) = input {
            input.depth = included_from.input.depth + 1;
            input.included_from = Some(included_from);
            self.inputs.push(Rc::new(input));
            self.inputs.last() // always Some
        } else {
            None
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let session = Rc::clone(&self.tu.session);
        for pass in &session.flags().passes {
            debug!(
                "TUCtx::run_one() tu alias {:?} running pass {:?}",
                self.tu.input().name,
                &pass
            );
            pass.run(self)?;
        }
        Ok(())
    }
}
