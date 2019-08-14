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

//! Translation Unit

use std::collections::HashMap;
use std::rc::Rc;

use crate::driver::{Driver, Input};
use crate::error::{ErrorKind, Result};
use crate::message::{Message, MessageKind};
use crate::token::{CharToken, Location, PPToken};

/// Translation Unit State
///
/// This is the primary intermediate state that is shared between passes.
/// Auxiliary state may be kept in [`TUCtx`].
///
/// [`TUCtx`]: ./struct.TUCtx.html
#[derive(Clone, Debug)]
pub enum TUState {
    CharTokens(Vec<crate::token::CharToken>),
    PPTokens(Vec<crate::token::PPToken>),
}

macro_rules! into_methods {
    ($(($method:ident, $variant:ident, $returns:ty)),+) => ($(
        pub fn $method(self) -> Result<$returns> {
            match self {
                TUState::$variant(val) => Ok(val),
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
        (into_chartokens, CharTokens, Vec<crate::token::CharToken>),
        (into_pptokens, PPTokens, Vec<crate::token::PPToken>)
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
    pub fn emit_message(&mut self, location: Location, kind: MessageKind) {
        self.messages.push(Message {
            location,
            kind: kind,
            include_history: Vec::new(),
        });
    }
}
