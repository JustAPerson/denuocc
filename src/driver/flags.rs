// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Compiler flags

use std::str::FromStr;

use lazy_static::lazy_static;
use log::{trace, warn};
use regex::Regex;

use crate::driver::Result;
use crate::passes::PASS_NAMES;

#[derive(Clone, Debug)]
pub struct Pass {
    pub name: String,
    pub args: Vec<String>,
}

impl FromStr for Pass {
    type Err = String;
    fn from_str(s: &str) -> Result<Pass, Self::Err> {
        lazy_static! {
            static ref REGEX: Regex =
                Regex::new(r"^([[:alpha:]][[:word:]]+)(\([^\)]+\))?$").unwrap();
        }
        let captures = REGEX
            .captures(s)
            .ok_or(format!("malformed pass specifier `{}`", s))?;

        // assuming unwrap is safe because this group is not optional
        let name = captures.get(1).unwrap().as_str().to_owned();

        if !PASS_NAMES.contains(&*name) {
            return Err(format!("unknown pass `{}`", name));
        }

        let args = {
            if let Some(arg_str) = captures.get(2).map(|m| m.as_str()) {
                let len = arg_str.len();
                let arg_str = &arg_str[1..len - 1]; // exclude parenthesis at each end
                arg_str.split(',').map(|s| s.trim().to_owned()).collect()
            } else {
                Vec::new()
            }
        };

        trace!("Pass::from_str() name = {:?} args = {:?}", name, args);

        Ok(Pass { name, args })
    }
}

/// Compiler flags
#[derive(Clone, Debug)]
pub struct Flags {
    pub passes: Vec<Pass>,
}

impl Flags {
    pub fn new() -> Flags {
        Flags { passes: Vec::new() }
    }

    pub fn process_clap_matches(&mut self, matches: &clap::ArgMatches) -> Result<()> {
        for pass_arg in matches.values_of_os("pass").into_iter().flatten() {
            let pass_arg = pass_arg
                .to_str()
                .ok_or_else(|| format!("non utf-8 argument for --pass flag: {:?}", pass_arg))?;
            let pass = Pass::from_str(pass_arg)
                .map_err(|e| format!("invalid argument for --pass flag: {}", e))?;
            self.passes.push(pass);
        }
        if self.passes.is_empty() {
            warn!("Flags::process_clap_matches() no passes");
        }

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
    use crate::driver::Driver;

    fn pass_parsing_case(input: &str, name: &str, args: &[&str]) {
        let pass: Pass = input.parse().unwrap();
        assert_eq!(pass.name, name);
        assert_eq!(pass.args, args);
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

    #[test]
    fn flags_pass_parsing_integration() {
        use crate::driver::ErrorKind;
        let mut driver = Driver::new();
        let error = driver
            .parse_args_from_str(&["--pass=nonexistent"])
            .unwrap_err();
        match error.kind() {
            ErrorKind::Generic(..) => { /* good */ },
            _ => panic!(), // bad
        }

        driver.parse_args_from_str(&["--pass=state_save"]).unwrap();
        assert_eq!(driver.flags.passes[0].name, "state_save");

        driver.clear_flags();
        driver
            .parse_args_from_str(&["--pass=state_print;state_save(a, b);state_print"])
            .unwrap();
        assert_eq!(driver.flags.passes[0].name, "state_print");
        assert_eq!(driver.flags.passes[1].name, "state_save");
        assert_eq!(driver.flags.passes[1].args[0], "a");
        assert_eq!(driver.flags.passes[1].args[1], "b");
        assert_eq!(driver.flags.passes[2].name, "state_print");
    }
}
