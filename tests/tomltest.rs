// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

// TODO revisit these unstable feature-attributes
#![feature(test, rustc_private)]

extern crate test;

use std::collections::HashMap;
use std::sync::Arc;

use denuocc::tu::TUState;
use denuocc::Driver;
use serde_derive::Deserialize;
use test::{TestDesc, TestDescAndFn, TestFn, TestName, TestType};
use toml::Spanned;

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResultsCompare {
    AssertChartokensEqual,
    AssertPptokensLooseEqual,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResultsPrint {
    ChartokensToString,
    PptokensToString,
}

#[derive(Debug, Deserialize)]
struct Config {
    suites: HashMap<String, Suite>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ShouldPanic {
    Bool(bool),
    Message(String),
}

impl std::convert::From<crate::ShouldPanic> for test::ShouldPanic {
    fn from(sp: crate::ShouldPanic) -> test::ShouldPanic {
        match sp {
            ShouldPanic::Bool(true) => test::ShouldPanic::Yes,
            ShouldPanic::Bool(false) => test::ShouldPanic::No,
            ShouldPanic::Message(s) => {
                let s = s.into_boxed_str();
                let s = Box::leak::<'static>(s);
                test::ShouldPanic::YesWithMessage(s)
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct Suite {
    passes: Vec<String>,
    results_compare: ResultsCompare,
    results_print: Option<ResultsPrint>,
    cases: Vec<Case>,

    #[serde(skip)]
    filename: std::path::PathBuf,
}

#[derive(Debug, Deserialize)]
struct Case {
    input: Spanned<String>,
    output: Option<String>,
    messages: Option<Vec<String>>,
    ignored: Option<bool>,
    should_panic: Option<ShouldPanic>,

    #[serde(skip)]
    line: usize,
}

impl Case {
    fn run_input(&self, suite: &Suite) -> TUState {
        let mut driver = Driver::new();
        driver.add_input_str("<case>", self.input.get_ref());

        for pass in &suite.passes {
            let arg = format!("--pass={}", pass);
            driver.parse_args_from_str(&[arg]).unwrap();
        }

        let mut tu = driver.run_one("<case>").unwrap();
        let state = tu.take_state().unwrap();
        let messages = tu
            .take_messages()
            .into_iter()
            .map(|m| format!("{}", m))
            .collect::<Vec<_>>();

        self.print_result(suite, &state);

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
            driver.parse_args_from_str(&[arg]).unwrap();
        }

        let mut tu = driver.run_one("<case>").unwrap();
        let state = tu.take_state().unwrap();
        let messages = tu.take_messages();

        self.print_result(suite, &state);

        if messages.len() > 0 {
            panic!("output of case had unexpected messages: \n{:?}", messages);
        }

        state
    }

    fn compare_input_output(&self, suite: &Suite, input: &TUState, output: &TUState) {
        use denuocc::front::token::{CharToken, PPToken};
        use ResultsCompare::*;

        match suite.results_compare {
            AssertChartokensEqual => {
                let input = input.as_chartokens().unwrap();
                let output = output.as_chartokens().unwrap();
                CharToken::assert_equal(input, output);
            },
            AssertPptokensLooseEqual => {
                let input = input.as_pptokens().unwrap();
                let output = output.as_pptokens().unwrap();
                PPToken::assert_loose_equal(input, output);
            },
        }
    }

    fn print_result(&self, suite: &Suite, result: &TUState) {
        use denuocc::front::token::{CharToken, PPToken};
        use ResultsPrint::*;

        if suite.results_print.is_none() {
            return;
        }

        match suite.results_print.unwrap() {
            ChartokensToString => {
                println!("{}", CharToken::to_string(result.as_chartokens().unwrap()))
            },
            PptokensToString => println!("{}", PPToken::to_string(result.as_pptokens().unwrap())),
        }
    }

    fn run(self, suite: &Suite) {
        let input_result = self.run_input(suite);

        if self.output.is_some() {
            let output_result = self.run_output(suite);
            self.compare_input_output(suite, &input_result, &output_result);
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

        // want test cases to sort by number correctly, so we need to zero-pad
        // the case number. first need to calculate the number of necessary
        // digits
        let width = (cases.len() as f32).log10().ceil() as usize;

        let mut index = 0;
        for mut case in cases {
            case.line = get_linenumber(&content, case.input.start());

            tests.push(TestDescAndFn {
                desc: TestDesc {
                    name: TestName::DynTestName(format!(
                        "{:?} suite={} case={:0width$} line={}",
                        &filename.as_os_str(),
                        &name,
                        index,
                        case.line,
                        width = width,
                    )),
                    ignore: case.ignored.unwrap_or(false),
                    should_panic: case
                        .should_panic
                        .clone()
                        .unwrap_or(ShouldPanic::Bool(false))
                        .into(),
                    allow_fail: false,
                    test_type: TestType::UnitTest,
                },
                testfn: TestFn::DynTestFn({
                    let suite = Arc::clone(&suite);
                    Box::new(move || {
                        case.run(&suite);
                    })
                }),
            });

            index += 1;
        }
    }

    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .is_test(true)
        .init();

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

    let args: Vec<String> = std::env::args().collect();
    test::test_main(args.as_slice(), tests, None);

    Ok(())
}
