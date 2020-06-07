// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Compiler passes

use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::driver::{ErrorKind, Result};
use crate::front;
use crate::tu::TUCtx;

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
            erase("state_print", &state::StatePrint::from_args),
            erase("state_print_debug", &state::StatePrintDebug::from_args),
            erase("state_save", &state::StateSave::from_args),
            erase("state_write", &state::StateWrite::from_args),
            erase("state_write_debug", &state::StateWriteDebug::from_args),
            erase("state_read_input", &state::StateReadInput::from_args),
            erase("phase1", &front::passes::Phase1::from_args),
            erase("phase2", &front::passes::Phase2::from_args),
            erase("phase3", &front::passes::Phase3::from_args),
            erase("phase4", &front::passes::Phase4::from_args),
            erase("phase5", &front::passes::Phase5::from_args),
            erase("phase6", &front::passes::Phase6::from_args),
        ].iter().map(|(s, c)| (*s, *c)).collect()
    };
}

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

pub mod state {
    use super::*;

    declare_pass!(
        /// Pretty-print TUCtx's primary state to stderr
        state_print => pub struct StatePrint {}
    );
    impl Pass for StatePrint {
        fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
            let state = tuctx.get_state()?;
            eprintln!("{}", state);
            Ok(())
        }
    }

    declare_pass!(
        /// Debug-print TUCtx's primary state to stderr
        state_print_debug => pub struct StatePrintDebug {}
    );
    impl Pass for StatePrintDebug {
        fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
            let state = tuctx.get_state()?;
            eprintln!("{:#?}", state);
            Ok(())
        }
    }

    declare_pass!(
        /// Save the current primary state for later access by
        /// [`TUCtx::saved_states()`][denuocc::tu::TUCtx::saved_states]
        state_save => pub struct StateSave {
            pub name: String
        }
    );
    impl Pass for StateSave {
        fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
            tuctx.save_state(&self.name)
        }
    }

    declare_pass!(
        /// Pretty-print TUCtx's primary state to file
        state_write => pub struct StateWrite {
            pub filename: String
        }
    );
    impl Pass for StateWrite {
        fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
            use std::io::Write;

            let state = tuctx.get_state()?;
            std::fs::File::open(&self.filename)
                .and_then(|mut f| write!(f, "{}", state))
                .map_err(|error| ErrorKind::OutputFileError {
                    filename: self.filename.to_owned(),
                    error,
                })?;

            Ok(())
        }
    }

    declare_pass!(
        /// Debug-print TUCtx's primary state to file
        state_write_debug => pub struct StateWriteDebug {
            pub filename: String
        }
    );
    impl Pass for StateWriteDebug {
        fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
            use std::io::Write;
            let state = tuctx.get_state()?;
            std::fs::File::open(&self.filename)
                .and_then(|mut f| write!(f, "{:#?}", state))
                .map_err(|error| ErrorKind::OutputFileError {
                    filename: self.filename.to_owned(),
                    error,
                })?;

            Ok(())
        }
    }

    declare_pass!(
        /// The entry point of every pass group
        ///
        /// Reads the specified input for this translation unit
        state_read_input => pub struct StateReadInput {}
    );
    impl Pass for StateReadInput {
        fn run(&self, tuctx: &mut TUCtx) -> Result<()> {
            use crate::front::token::CharToken;
            let input = tuctx.input();
            let tokens = CharToken::from_input(input);
            tuctx.set_state(crate::tu::TUState::CharTokens(tokens));

            Ok(())
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
