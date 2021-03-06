// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Internal types used by every frontend or backend

mod error;
mod flags;
mod message;

pub use error::{Error, ErrorKind, Result};
pub use flags::{generate_clap_args, Flags};
pub use message::{Message, Severity};
