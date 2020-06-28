// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Compiler passes
//!
//! This module declares [Pass][] objects so they may be used during the compilation process.
//!
//! Compilation largely follows a linear series of transformation, where the
//! output of one operation is used as the input of another transformation.
//!
//! The compiler will supply a default set of passes to use depending on the
//! configuration. The user may also override the defaults by using the `--pass`
//! command line flag. Both of these cases are handled in
//! [`Flags::process_clap_matches()`][pcm].
//!
//! [pcm]: crate::driver::flags::Flags::process_clap_matches

use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::driver::{ErrorKind, Result};
use crate::tu::TUCtx;

pub mod front;
pub mod internal;

/// Step in the compilation process
///
/// See the [module documentation][crate::passes] for more details.
pub trait Pass: std::fmt::Debug + ClonePass {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()>;
}

/// Type system hack
///
/// Rust does not have a very good way to express that a trait object should be
/// cloneable.
pub trait ClonePass {
    fn clone_pass(&self) -> Box<dyn Pass>;
}
impl<T: Pass + Clone + 'static> ClonePass for T {
    fn clone_pass(&self) -> Box<dyn Pass> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn Pass> {
    fn clone(&self) -> Self {
        self.clone_pass()
    }
}

/// A way of constructing a pass
///
/// This is implemented for the trait `Fn(&[&str]) -> Result<Box<dyn Pass>>`,
/// meaning any function of that type implement this trait.
pub trait ConstructPass: Send + Sync {
    fn construct(&self, args: &[&str]) -> Result<Box<dyn Pass>>;
}
impl<T: Send + Sync + Fn(&[&str]) -> Result<Box<dyn Pass>>> ConstructPass for T {
    fn construct(&self, args: &[&str]) -> Result<Box<dyn Pass>> {
        self(args)
    }
}
lazy_static! {
    /// A map from pass names to constructors
    pub static ref PASS_CONSTRUCTORS: HashMap<&'static str, &'static dyn ConstructPass> = {
        // Seems rustc has trouble type erasing these Fn implementations
        fn erase(
            s: &'static str,
            c: &'static dyn ConstructPass,
        ) -> (&'static str, &'static dyn ConstructPass) {
            (s, c)
        }
        [
            erase("state_print", &internal::StatePrint::from_args),
            erase("state_print_debug", &internal::StatePrintDebug::from_args),
            erase("state_save", &internal::StateSave::from_args),
            erase("state_write", &internal::StateWrite::from_args),
            erase("state_write_debug", &internal::StateWriteDebug::from_args),
            erase("state_read_input", &internal::StateReadInput::from_args),
            erase("phase1", &front::Phase1::from_args),
            erase("phase2", &front::Phase2::from_args),
            erase("phase3", &front::Phase3::from_args),
            erase("phase4", &front::Phase4::from_args),
            erase("phase5", &front::Phase5::from_args),
            erase("phase6", &front::Phase6::from_args),
        ].iter().map(|(s, c)| (*s, *c)).collect()
    };
}

/// Declare compiler [`Pass`][crate::passes::Pass] structs
#[macro_export]
macro_rules! declare_pass {
    {
        $(#[$meta:meta])*
        $alias:ident => pub struct $name:ident {
            $( pub $field:ident : $type:ty);*
        }
    } => {
        $(#[$meta])*
        #[derive(Clone, Debug)]
        pub struct $name {
            $(pub $field: $type);*
        }
        impl $name {
            pub fn from_args(args: &[&str]) -> Result<Box<dyn Pass>> {
                const fn count() -> usize {
                    let mut _c = 0;
                    // must use a variable, so throw away the string literal
                    $(_c += 1; stringify!($field); )*
                    return _c;
                }
                crate::passes::helper::args_count(stringify!($alias), args, count())?;
                let mut _index = 0;
                $(
                    let $field: $type =
                    crate::passes::helper::args_get(stringify!($alias),
                    args, _index, stringify!($type))?;
                    _index += 1;
                );*
                Ok(Box::new($name { $($field),* }))
            }
        }
    }
}

/// Functions used internally by [`declare_pass!()`][macro@declare_pass].
///
/// Use that macro instead of these directly.
pub mod helper {
    use super::*;

    /// Checks the number of arguments
    ///
    /// Returns a [`PassArgsArity`][crate::ErrorKind::PassArgsArity] error if
    /// `args.len() ! = expects`.
    pub fn args_count(name: impl Into<String>, args: &[&str], expects: usize) -> Result<()> {
        if args.len() == expects {
            Ok(())
        } else {
            Err(ErrorKind::PassArgsArity {
                pass_name: name.into(),
                expects,
                got: args.len(),
            }
            .into())
        }
    }

    /// Parses and returns the corresponding argument
    ///
    /// Returns a [`PassArgsType`][crate::ErrorKind::PassArgsType]
    /// error if parsing fails.
    pub fn args_get<T: std::str::FromStr>(
        name: impl Into<String>,
        args: &[&str],
        index: usize,
        expects: &'static str,
    ) -> Result<T> {
        args[index].parse::<T>().map_err(|_| {
            ErrorKind::PassArgsType {
                pass_name: name.into(),
                index,
                expects,
                got: args[index].to_owned(),
            }
            .into()
        })
    }
}
