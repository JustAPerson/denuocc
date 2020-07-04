// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Errors or warnings resulting from source code
//!
//! Not to be confused with [`Error`][crate::Error] which is for run time errors
//! in the compiler, mostly user errors.

/// A remark about the source code being translated
///
/// Each front end may provide their own implementation, potentially with extra
/// features. At a minimum, every message should have a severity and a short
/// text description. The [`Display`][std::fmt::Display] implementation should
/// provide this short text description. Specific implementations may provide
/// more rich descriptions.
pub trait Message: std::fmt::Display {
    fn severity(&self) -> Option<Severity>;
}

/// How severe a [`Message`][Message] is
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    /// Compilation cannot succeed with an error
    Error,

    /// Compilation can succeed with a warning, but it should be displayed to
    /// the user
    Warning,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match *self {
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
