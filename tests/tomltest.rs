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

// TODO revisit these unstable feature-attributes
#![feature(test, rustc_private)]

extern crate test;

use std::collections::HashMap;
use std::sync::Arc;

use denuocc::Driver;
use denuocc::tu::TUState;
use serde_derive::Deserialize;
use test::{ShouldPanic, TestDesc, TestDescAndFn, TestFn, TestName};
use toml::Spanned;

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum OutputType {
    PPTokens,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OutputCompare {
    AssertPptokensLooseEqual,
}

#[derive(Debug, Deserialize)]
struct Config {
    suites: HashMap<String, Suite>,
}

#[derive(Debug, Deserialize)]
struct Suite {
    passes: Vec<String>,
    output_compare: OutputCompare,
    cases: Vec<Case>,

    #[serde(skip)]
    filename: std::path::PathBuf,
}

#[derive(Debug, Deserialize)]
struct Case {
    input: Spanned<String>,
    output: Option<String>,
    messages: Option<Vec<String>>,

    #[serde(skip)]
    line: usize,
}

impl Case {
    fn run_input(&self, suite: &Suite) -> TUState {
        let mut driver = denuocc::Driver::new();
        driver.add_input_str("<case>", self.input.get_ref());

        for pass in &suite.passes {
            let arg = format!("--pass={}", pass);
            driver.parse_args_from_str(&[arg]).unwrap();
        }

        let mut tu = driver.run_one("<case>").unwrap();
        let state = tu.take_state().unwrap();
        let messages = tu.take_messages().into_iter().map(|m| format!("{}", m)).collect::<Vec<_>>();

        if let Some(ref expected_messages) = self.messages {
            assert_eq!(&messages, expected_messages);
        } else if !messages.is_empty() {
            // we did not expect any messages yet some were produced
            panic!("unexpected messages {:#?}", &messages);
        }

        state
    }

    fn run_output(&self, suite: &Suite) -> TUState {
        let mut driver = denuocc::Driver::new();
        driver.add_input_str("<case>", self.output.as_ref().unwrap());

        for pass in &suite.passes {
            let arg = format!("--pass={}", pass);
            driver.parse_args_from_str(&[arg]);
        }

        let mut tu = driver.run_one("<case>").unwrap();
        let state = tu.take_state().unwrap();
        let messages = tu.take_messages();

        assert_eq!(messages.len(), 0);

        state
    }

    fn compare_input_output(&self, suite: &Suite, input: TUState, output: TUState) {
        use OutputCompare::*;
        use denuocc::token::assert_pptokens_loose_equal;

        match suite.output_compare {
            AssertPptokensLooseEqual => {
                let input = input.into_pptokens().unwrap();
                let output = output.into_pptokens().unwrap();
                assert_pptokens_loose_equal(&input, &output);
            }
        }
    }


    fn run(self, suite: &Suite) {
        let input_result = self.run_input(suite);

        if self.output.is_some() {
            let output_result = self.run_output(suite);
            self.compare_input_output(suite, input_result, output_result);
        }
    }
}

fn get_linenumber(input: &str, characters: usize) -> usize {
    let mut line = 1;

    for c in input.chars().take(characters) {
        if c == '\n' {
            line += 1;
        }
    }

    return line;
}

fn read_toml(
    filename: &std::path::Path,
    tests: &mut Vec<TestDescAndFn>,
) -> Result<(), std::io::Error> {
    let content = std::fs::read_to_string(filename)?;
    let config: Config = toml::from_str(&content).unwrap();

    for (name, mut suite) in config.suites {
        // move cases out of Suite
        let mut cases = Vec::new();
        cases.append(&mut suite.cases);
        suite.filename = filename.to_owned();

        // no longer need to modify Suite, so make an Arc
        let suite = Arc::new(suite);

        let mut num_cases = 0;
        for mut case in cases {
            case.line = get_linenumber(&content, case.input.start());

            tests.push(TestDescAndFn {
                desc: TestDesc {
                    name: TestName::DynTestName(format!(
                        "{:?} suite={} case={} line={}",
                        &filename.as_os_str(), &name, num_cases, case.line
                    )),
                    ignore: false,
                    should_panic: ShouldPanic::No,
                    allow_fail: false,
                },
                testfn: TestFn::DynTestFn({
                    let suite = Arc::clone(&suite);
                    Box::new(move || {
                        case.run(&suite);
                    })
                }),
            });

            num_cases += 1;
        }
    }

    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    let mut tests = Vec::new();
    // PWD is project root

    for entry in std::fs::read_dir("tests")? {
        let entry = entry?;
        if entry.path().extension().unwrap() != "toml" {
            continue;
        }
        let filename = entry.path();

        read_toml(&filename, &mut tests)?;
    }

    let opts = test::parse_opts(&[]).unwrap().unwrap();
    test::run_tests_console(&opts, tests);

    Ok(())
}
