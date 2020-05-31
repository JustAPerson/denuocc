// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Runtime errors within the compiler
//!
//! For errors originating in the provided code being translated, see
//! [`Message`][crate::front::message::Message].

/// [`Result`][std::result::Result] but defaults to using denuocc's
/// [`Error`][Error]
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The different types of errors that can arise
#[derive(Debug)]
pub enum ErrorKind {
    Generic(String),
    ClapError(clap::Error),
    InputFileError {
        filename: String,
        error: std::io::Error,
    },
    OutputFileError {
        filename: String,
        error: std::io::Error,
    },

    TUStateAbsent,
    TUStateTypeError {
        current_type: &'static str,
        expected_type: &'static str,
    },

    PassArgsArity {
        pass_name: &'static str,
        expects: u32,
        got: u32,
    },
    PassArgsArityAtMost {
        pass_name: &'static str,
        most: u32,
        got: u32,
    },
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        use ErrorKind::*;
        match self {
            Generic(s) => write!(f, "{}", s),
            ClapError(e) => write!(f, "{}", e),
            InputFileError { filename, error } => {
                write!(f, "cannot read file `{}`: {}", filename, error)
            },
            OutputFileError { filename, error } => {
                write!(f, "cannot write file `{}`: {}", filename, error)
            },

            TUStateAbsent => write!(f, "no input state for pass"),
            TUStateTypeError {
                current_type,
                expected_type,
            } => write!(
                f,
                "mismatched input state for pass; got `{}`; expected `{}`",
                current_type, expected_type
            ),

            PassArgsArity {
                pass_name,
                expects,
                got,
            } => write!(
                f,
                "pass `{}` takes {} arguments; received {}",
                pass_name, expects, got
            ),
            PassArgsArityAtMost {
                pass_name,
                most,
                got,
            } => write!(
                f,
                "pass `{}` takes at most {} arguments; received {}",
                pass_name, most, got
            ),
        }
    }
}

#[derive(Debug)]
struct ErrorInterior {
    pub kind: ErrorKind,
    pub backtrace: backtrace::Backtrace,
}

/// An error wrapper
pub struct Error {
    interior: Box<ErrorInterior>,
}

impl Error {
    /// Return the kind of error
    pub fn kind(&self) -> &ErrorKind {
        &self.interior.kind
    }

    /// Return a backtrace from where this error originated
    pub fn backtrace(&self) -> &backtrace::Backtrace {
        &self.interior.backtrace
    }

    /// Print the error message and exit the process
    ///
    /// For most [ErrorKinds][ErrorKind], this will print the error to stderr
    /// and then exit with status code `2`. For specific
    /// [ErrorKind::ClapError][] instances corresponding to the `-h` and `-v`
    /// flags, this will print to stdout and exit with status code `0`.
    pub fn print_and_exit(&self) -> ! {
        use clap::ErrorKind::{HelpDisplayed, VersionDisplayed};
        if let ErrorKind::ClapError(ce) = &self.interior.kind {
            if [HelpDisplayed, VersionDisplayed].contains(&ce.kind) {
                println!("{}", ce.message);
                std::process::exit(0);
            }
        }
        eprintln!("error: {}", self);
        std::process::exit(2);
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.interior.kind)
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self.interior)
    }
}

impl std::convert::From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        let interior = Box::new(ErrorInterior {
            kind,
            backtrace: backtrace::Backtrace::new(),
        });
        Error { interior }
    }
}

impl std::convert::From<clap::Error> for Error {
    fn from(error: clap::Error) -> Error {
        ErrorKind::ClapError(error).into()
    }
}

impl std::convert::From<String> for Error {
    fn from(s: String) -> Error {
        ErrorKind::Generic(s).into()
    }
}
