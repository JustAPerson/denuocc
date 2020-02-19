// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Translation Unit

use std::collections::HashMap;
use std::rc::Rc;

use crate::driver::{Driver, Input};
use crate::error::{ErrorKind, Result};
use crate::front::location::Location;
use crate::front::message::{Message, MessageKind};
use crate::front::token::{CharToken, PPToken};

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

/// Translation Unit Context
#[derive(Clone, Debug)]
pub struct TUCtx<'a> {
    driver: &'a Driver,
    input: Rc<Input>,
    messages: Vec<Message>,
    state: Option<TUState>,
    saved_states: HashMap<String, Vec<TUState>>,
}

impl<'a> TUCtx<'a> {
    pub fn from_driver(driver: &'a Driver, name: &str) -> TUCtx<'a> {
        TUCtx {
            driver: driver,
            input: Rc::clone(
                driver
                    .inputs
                    .get(name)
                    .unwrap_or_else(|| panic!("input name not found; got `{}`", name)),
            ),
            messages: Vec::new(),
            state: None,
            saved_states: HashMap::new(),
        }
    }

    /// Returns the underlying compilation driver
    pub fn driver(&self) -> &'a Driver {
        self.driver
    }

    /// Returns the corresponding input for this unit
    pub fn input(&self) -> &Rc<Input> {
        &self.input
    }

    /// Returns the states associated with the given name
    ///
    /// States are saved by the [`save_state`] method, which is implicitly used
    /// in the `state_save` pass.
    ///
    /// [`save_state`]: struct.TUCtx.html#save_state
    pub fn saved_states(&self, name: &str) -> &Vec<TUState> {
        self.saved_states
            .get(name)
            .unwrap_or_else(|| panic!("No state named `{}` found", name))
    }

    /// Saves the current state, associating it with the given name
    ///
    /// Implicitly used in the `state_save` pass.
    pub fn save_state(&mut self, name: &str) -> Result<()> {
        let state = self.get_state()?.clone();
        let entry = self
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

    /// Move messages out of this context
    pub fn take_messages(&mut self) -> Vec<Message> {
        std::mem::replace(&mut self.messages, Vec::new())
    }

    /// Emit an error to this translation unit's list
    pub fn emit_message(&mut self, location: impl Into<Location>, kind: MessageKind) {
        self.messages.push(Message {
            location: location.into(),
            kind: kind,
        });
    }
}
