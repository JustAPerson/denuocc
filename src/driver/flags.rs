// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Compiler flags

use lazy_static::lazy_static;
use log::{trace, warn};
use regex::Regex;

use crate::driver::Result;
use crate::passes::Pass;
use crate::passes::PASS_CONSTRUCTORS;

lazy_static! {
    static ref REGEX: Regex = Regex::new(r"^([[:alpha:]][[:word:]]+)(\([^\)]+\))?$").unwrap();
}

fn lex_pass_args(specifier: &str) -> Result<(&str, Vec<&str>)> {
    let captures = REGEX
        .captures(specifier)
        .ok_or(format!("malformed pass specifier `{}`", specifier))?;

    // assuming unwrap is safe because this group is not optional
    let name = captures.get(1).unwrap().as_str();

    let args = {
        if let Some(arg_str) = captures.get(2).map(|m| m.as_str()) {
            let len = arg_str.len();
            let arg_str = &arg_str[1..len - 1]; // exclude parenthesis at each end
            arg_str.split(',').map(|s| s.trim()).collect()
        } else {
            Vec::new()
        }
    };

    Ok((name, args))
}

fn parse_pass(specifier: &std::ffi::OsStr) -> Result<Box<dyn Pass>> {
    let specifier = specifier
        .to_str()
        .ok_or_else(|| format!("non utf-8 argument for --pass flag: {:?}", specifier))?;

    let (name, args) = lex_pass_args(specifier)?;
    trace!("Pass::from_str() name = {:?} args = {:?}", name, args);

    let constructor = PASS_CONSTRUCTORS
        .get(name)
        .ok_or_else(|| format!("unknown pass `{}`", name))?;
    (constructor)(&*args)
}

/// Compiler flags
#[derive(Clone, Debug)]
pub struct Flags {
    pub passes: Vec<Box<dyn Pass>>,
}

impl Flags {
    pub fn new() -> Flags {
        Flags { passes: Vec::new() }
    }

    pub fn process_clap_matches(&mut self, matches: &clap::ArgMatches) -> Result<()> {
        // use requested passes or use defaults?
        if matches.is_present("pass") {
            for pass_arg in matches.values_of_os("pass").into_iter().flatten() {
                let pass = parse_pass(pass_arg)
                    .map_err(|e| format!("invalid argument for --pass flag: {}", e))?;
                self.passes.push(pass);
            }
        } else {
            warn!("Flags::process_clap_matches() no passes");
        }
        assert!(!self.passes.is_empty());

        Ok(())
    }
}

impl std::default::Default for Flags {
    fn default() -> Flags {
        let flags = Flags::new();
        flags
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn pass_parsing_case(input: &str, name: &str, args: &[&str]) {
        let lexed = lex_pass_args(input).unwrap();
        assert_eq!(lexed.0, name);
        assert_eq!(lexed.1, args);
    }

    #[test]
    #[should_panic]
    fn flags_pass_parsing_nonexistent() {
        pass_parsing_case("nonexistent", "", &[]);
    }

    #[test]
    #[should_panic]
    fn flags_pass_parsing_mismatched_args() {
        pass_parsing_case("state_save(1,two)", "state_save", &["one", "2"]);
    }

    #[test]
    fn flags_pass_parsing() {
        pass_parsing_case("state_save", "state_save", &[]);
        pass_parsing_case("state_save(1)", "state_save", &["1"]);
        pass_parsing_case("state_save(1,two)", "state_save", &["1", "two"]);
    }
}
