// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Translation Unit Context

use std::rc::Rc;

use log::{debug, info};

use crate::core::{ErrorKind, Result, Severity};
use crate::front::c::input::{IncludedFrom, Input};
use crate::front::c::message::{Message, MessageKind};
use crate::front::c::token::{CharToken, MacroInvocation, PPToken, TokenOrigin};
use crate::front::c::tu::TranslationUnit;

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
    pub(super) tu: &'a mut TranslationUnit,
    pub(super) inputs: Vec<Rc<Input>>,
    pub(super) state: Option<TUState>,
    pub(super) macro_invocations: Vec<MacroInvocation>,

    fatal_error: bool,
}

impl<'a> TUCtx<'a> {
    pub fn from_tu(tu: &'a mut TranslationUnit) -> TUCtx<'a> {
        let mut inputs = Vec::new();
        inputs.push(Rc::clone(&tu.input));

        TUCtx {
            tu,
            inputs,
            state: None,
            macro_invocations: Vec::new(),

            fatal_error: false,
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
    pub fn emit_message(&mut self, origin: impl Into<TokenOrigin>, kind: MessageKind) {
        let origin = origin.into();
        info!(
            "TUCTx::emit_message() kind {:?} origin {:?}",
            &kind, &origin
        );
        if kind.severity() == Severity::Fatal {
            self.fatal_error = true;
        }
        self.tu.messages.push(Message {
            kind,
            origin,
            children: None,
            extra: None,
        });
    }

    pub fn emit_message_with_children(
        &mut self,
        origin: impl Into<TokenOrigin>,
        kind: MessageKind,
        children: Vec<impl Into<Message>>,
    ) {
        let origin = origin.into();
        let children = children
            .into_iter()
            .map(|p| p.into())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        info!(
            "TUCtx::emit_message_with_children() kind {:?} origin {:?} children {:?}",
            &kind, &origin, &children
        );
        if kind.severity() == Severity::Fatal {
            self.fatal_error = true;
        }
        self.tu.messages.push(Message {
            kind,
            origin,
            children: Some(children),
            extra: None,
        })
    }

    pub fn add_macro_invocation(&mut self, invocation: MacroInvocation) -> u32 {
        let id = self.macro_invocations.len();
        self.macro_invocations.push(invocation);
        id as u32
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
            input.id = self.inputs.len() as u32;
            self.inputs.push(Rc::new(input));
            self.inputs.last() // always Some
        } else {
            None
        }
    }

    pub fn run(&mut self) -> Result<bool> {
        let session = Rc::clone(&self.tu.session);
        let passes = &session.flags().passes;
        for pass in passes {
            debug!(
                "TUCtx::run() tu alias {:?} running pass {:?}",
                self.tu.input().name,
                &pass
            );
            pass.run(self)?;
            debug!("TUCtx::run() fatal {}", self.fatal_error);
            if self.fatal_error {
                info!("TUCtx::run() stopping because of fatal error");
                break;
            }
        }
        if passes.len() > 0 {
            self.save_state("<final>").unwrap();
        }

        self.enrich_messages();
        Ok(self.fatal_error)
    }

    fn enrich_messages(&mut self) {
        // Gotta appease borrowck
        let mut messages = std::mem::take(&mut self.tu.messages);
        messages.iter_mut().for_each(|m| m.enrich(self));
        self.tu.messages = messages;
    }
}
