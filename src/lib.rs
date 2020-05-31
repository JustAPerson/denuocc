// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Denuo C Compiler

pub mod artifact;
pub mod driver;
pub mod passes;
pub mod tu;
pub mod util;

pub mod front;

pub use crate::driver::{Driver, Error, ErrorKind, Result};
