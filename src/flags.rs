// Copyright (C) 2019 Jason Priest
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation, either  version 3 of the  License, or (at your  option) any later
// version.
//
// This program is distributed  in the hope that it will  be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR  A PARTICULAR  PURPOSE.  See  the GNU  General  Public  License for  more
// details.
//
// You should have received a copy of  the GNU General Public License along with
// this program. If not, see <https://www.gnu.org/licenses/>.

//! Compiler flags

use lazy_static::lazy_static;
use regex::Regex;

use crate::error::Result;
use crate::passes::PASS_NAMES;

#[derive(Clone, Debug)]
pub struct Pass {
    pub name: String,
    pub args: Vec<String>,
}

impl std::str::FromStr for Pass {
    type Err = ();
    fn from_str(s: &str) -> Result<Pass, Self::Err> {
        lazy_static! {
            static ref REGEX: Regex =
                Regex::new(r"^([[:alpha:]][[:word:]]+)(\([^\)]+\))?$").unwrap();
        }
        let captures = REGEX.captures(s).ok_or(())?;

        let name = captures.get(1).ok_or(())?.as_str().to_owned();
        if !PASS_NAMES.contains(&*name) {
            return Err(());
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

    pub fn process_clap_matches(&mut self, matches: clap::ArgMatches) -> Result<()> {
        use clap::values_t;

        if matches.is_present("pass") {
            self.passes.append(&mut values_t!(matches, "pass", Pass)?);
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
        use crate::error::ErrorKind;
        let mut driver = Driver::new();
        let error = driver
            .parse_args_from_str(&["--pass=nonexistent"])
            .unwrap_err();
        match *error {
            ErrorKind::ClapError(..) => { /* good */ }
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
