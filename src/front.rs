// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Compiler front-end; everything about C syntax

pub mod input;
pub mod lexer;
pub mod location;
pub mod message;
pub mod minor;
pub mod passes;
pub mod preprocessor;
pub mod token;
