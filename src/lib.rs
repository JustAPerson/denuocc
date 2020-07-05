// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Denuo C Compiler

pub mod core;
pub mod driver;
#[macro_use]
pub mod passes;
pub mod session;
pub mod tu;
pub mod util;

pub mod front;

pub use crate::core::{Error, ErrorKind, Result};
pub use crate::driver::Driver;
pub use crate::session::Session;
