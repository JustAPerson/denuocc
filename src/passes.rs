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

//! Compiler passes

use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;

use crate::error::{ErrorKind, Result};
use crate::tu::TUCtx;

pub mod preprocess_phase1;
pub mod preprocess_phase2;
pub mod preprocess_phase3;
pub mod preprocess_phase4;
pub mod state;

/// The type of a pass implementation function
pub type PassFn = fn(&mut TUCtx, &[String]) -> Result<()>;

lazy_static! {
    /// A set of all pass names. Used for quick verification
    pub static ref PASS_NAMES: HashSet<&'static str> = {
        let mut h = HashSet::new();
        for (name, _) in PASSES {
            h.insert(*name);
        }
        h
    };
    /// A map from pass names to the functions that implement them
    pub static ref PASS_FUNCTIONS: HashMap<&'static str, PassFn> = {
        let mut h = HashMap::new();
        for (name, f) in PASSES {
            h.insert(*name, *f);
        }
        h
    };
}

/// List of compilation passes
///
/// The 0th element is the name of the pass, as used in the command line.
/// The 1st element is a pointer to the function implementing the pass.
// #[rustfmt::skip]
pub static PASSES: &[(&str, PassFn)] = &[
    ("state_print", state::state_print),
    ("state_print_debug", state::state_print_debug),
    ("state_save", state::state_save),
    ("state_write", state::state_write),
    ("state_write_debug", state::state_write_debug),
    ("state_read_input", state::state_read_input),
    ("preprocess_phase1", preprocess_phase1::preprocess_phase1),
    ("preprocess_phase2", preprocess_phase2::preprocess_phase2),
    ("preprocess_phase3", preprocess_phase3::preprocess_phase3),
    ("preprocess_phase4", preprocess_phase4::preprocess_phase4),
];

/// Useful functions for writing passes
pub mod helper {
    use super::*;

    /// Asserts that the pass was given the correct number of arguments or
    /// construct an appropriate error.
    pub fn args_assert_count(pass_name: &'static str, args: &[String], expects: u32) -> Result<()> {
        if args.len() == expects as usize {
            Ok(())
        } else {
            Err(ErrorKind::PassArgsArity {
                pass_name,
                expects,
                got: args.len() as u32,
            }
            .into())
        }
    }
}

// /// Useful functions that encapsulate setting up and using different passes
// pub mod wrappers {
//     pub fn wrap_phases1to3() {

//     }
//     pub fn wrap_phases1to4() {
//     }
// }

// pub(crate) fn args_assert_at_most(
//     pass_name: &'static str,
//     args: &[String],
//     most: u32,
// ) -> Result<()> {
//     if args.len() <= most as usize {
//         Ok(())
//     } else {
//         Err(ErrorKind::PassArgsArityAtMost {
//             pass_name,
//             most,
//             got: args.len() as u32,
//         }
//         .into())
//     }
// }
