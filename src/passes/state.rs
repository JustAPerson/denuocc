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

use crate::error::{ErrorKind, Result};
use crate::passes::helper::args_assert_count;
use crate::token::CharToken;
use crate::tu::{TUCtx, TUState};

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
    args_assert_count("state_read_input", args, 0)?;

    let input = tuctx.input();
    let tokens = CharToken::from_input(input);
    tuctx.set_state(TUState::CharTokens(tokens));

    Ok(())
}

// TODO test both debug styles
// TODO test assert_args_empty here
// TODO test both state_write
