// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! The different representations of a token at different phases of compilation
//!
//! A token is the minimum atomic unit under consideration during a particular
//! phase of compilation.

pub mod char_token;
pub mod preprocessor_token;

pub use self::char_token::CharToken;
pub use self::preprocessor_token::{PPToken, PPTokenKind};
