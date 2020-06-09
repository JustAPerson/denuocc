// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Compiler passes

use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::driver::{ErrorKind, Result};
use crate::tu::TUCtx;

pub mod front;
pub mod internal;

pub trait Pass: std::fmt::Debug + ClonePass {
    fn run(&self, tuctx: &mut TUCtx) -> Result<()>;
}

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

type Constructor = dyn Send + Sync + Fn(&[&str]) -> Result<Box<dyn Pass>>;
lazy_static! {
    /// A map from pass names to constructors
    pub static ref PASS_CONSTRUCTORS: HashMap<&'static str, &'static Constructor> = {
        // Seems rustc has trouble type erasing these Fn implementations
        fn erase(s: &'static str, c: &'static Constructor) -> (&'static str, &'static Constructor) {
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


/// Useful functions for writing passes
pub mod helper {
    use super::*;

    /// Asserts that the pass was given the correct number of arguments or
    /// construct an appropriate error.
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
