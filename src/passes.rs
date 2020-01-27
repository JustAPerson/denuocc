// Copyright (C) 2019 - 2020 Jason Priest
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

use crate::front;
use crate::error::{ErrorKind, Result};
use crate::tu::TUCtx;

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

/// List of available compilation passes
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
    ("phase1", front::passes::phase1),
    ("phase2", front::passes::phase2),
    ("phase3", front::passes::phase3),
    ("phase4", front::passes::phase4),
];

pub mod state {
    use super::*;
    use super::helper::args_assert_count;

    /// Pretty-print TUCtx's primary state to stderr
    pub fn state_print<'t>(tuctx: &mut TUCtx<'t>, args: &[String]) -> Result<()> {
        args_assert_count("state_print", args, 0)?;

        let state = tuctx.get_state()?;
        eprintln!("{}", state);

        Ok(())
    }

    /// Debug-print TUCtx's primary state to stderr
    pub fn state_print_debug<'t>(tuctx: &mut TUCtx<'t>, args: &[String]) -> Result<()> {
        args_assert_count("state_print_debug", args, 0)?;

        let state = tuctx.get_state()?;
        eprintln!("{:#?}", state);

        Ok(())
    }

    /// Save the current primary state for later access by
    /// [`TUCtx::saved_states()`](../../tu/struct.TUCtx.html#method.saved_states)
    pub fn state_save<'t>(tuctx: &mut TUCtx<'t>, args: &[String]) -> Result<()> {
        args_assert_count("state_save", args, 1)?;

        tuctx.save_state(&args[0])
    }

    /// Pretty-print TUCtx's primary state to file
    pub fn state_write<'t>(tuctx: &mut TUCtx<'t>, args: &[String]) -> Result<()> {
        use std::io::Write;

        args_assert_count("state_write", args, 0)?;

        let state = tuctx.get_state()?;
        let filename = &args[0];
        std::fs::File::open(filename)
            .and_then(|mut f| write!(f, "{}", state))
            .map_err(|error| ErrorKind::OutputFileError {
                filename: filename.to_owned(),
                error,
            })?;

        Ok(())
    }

    /// Debug-print TUCtx's primary state to file
    pub fn state_write_debug<'t>(tuctx: &mut TUCtx<'t>, args: &[String]) -> Result<()> {
        use std::io::Write;

        args_assert_count("state_write_debug", args, 0)?;

        let state = tuctx.get_state()?;
        let filename = &args[0];
        std::fs::File::open(filename)
            .and_then(|mut f| write!(f, "{:#?}", state))
            .map_err(|error| ErrorKind::OutputFileError {
                filename: filename.to_owned(),
                error,
            })?;

        Ok(())
    }

    pub fn state_read_input<'t>(tuctx: &mut TUCtx<'t>, args: &[String]) -> Result<()> {
        use crate::token::CharToken;
        args_assert_count("state_read_input", args, 0)?;

        let input = tuctx.input();
        let tokens = CharToken::from_input(input);
        tuctx.set_state(crate::tu::TUState::CharTokens(tokens));

        Ok(())
    }
}

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
