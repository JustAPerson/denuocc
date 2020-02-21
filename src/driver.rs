// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Denuocc front-end

use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{ErrorKind, Result};
use crate::flags::Flags;
use crate::front::message::Message;
use crate::passes::PASS_FUNCTIONS;
use crate::target;
use crate::tu::TUCtx;

/// A translation unit input
#[derive(Clone, Debug)]
pub struct Input {
    pub name: String,
    pub content: String,
    pub is_file: bool,
}

/// Main interface for invoking denuocc
#[derive(Debug)]
pub struct Driver {
    /// A map from input names to their contents
    pub inputs: HashMap<String, Rc<Input>>,
    pub flags: Flags,

    pub messages: HashMap<String, Vec<Message>>,
    pub target: Box<dyn target::Target>,
}

impl Driver {
    pub fn new() -> Self {
        Driver::default()
    }

    /// Read command-line arguments from process environment
    pub fn parse_args_from_env(&mut self) -> Result<()> {
        let app = generate_clap(true);
        self.process_clap_matches(app.get_matches())
    }

    /// Read command-line arguments from string
    ///
    /// Do not include the binary name as first argument
    pub fn parse_args_from_str(
        &mut self,
        input: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>,
    ) -> Result<()> {
        let app = generate_clap(false).setting(clap::AppSettings::NoBinaryName);
        self.process_clap_matches(app.get_matches_from_safe(input)?)
    }

    fn process_clap_matches(&mut self, matches: clap::ArgMatches) -> Result<()> {
        if let Some(files) = matches.values_of("FILES") {
            for file in files {
                self.add_input_file(file)?;
            }
        }
        self.flags.process_clap_matches(matches)?;
        Ok(())
    }

    pub fn clear_flags(&mut self) {
        self.flags = Flags::default();
    }

    /// Adds the contents of the given path to the list of input translation
    /// units
    pub fn add_input_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let stdin_path: &std::path::Path = "-".as_ref();

        let name;
        let input;

        if path.as_ref() == stdin_path {
            use std::io::Read;

            name = "<stdin>".to_owned();
            let mut content = String::new();
            std::io::stdin()
                .lock()
                .read_to_string(&mut content)
                .map_err(|e| ErrorKind::InputFileError {
                    filename: name.clone(),
                    error: e,
                })?;
            input = Input {
                name: name.clone(),
                content: content,
                is_file: false,
            };
        } else {
            name = path.as_ref().to_string_lossy().into_owned();
            let content =
                std::fs::read_to_string(&name).map_err(|e| ErrorKind::InputFileError {
                    filename: name.to_owned(),
                    error: e,
                })?;
            input = Input {
                name: name.clone(),
                content: content,
                is_file: true,
            };
        }

        self.inputs.insert(name, Rc::new(input));

        Ok(())
    }

    /// Adds the given string to list of input translation units
    ///
    /// `name` must be wrapped in angle brackets (<>) to help distinguish from
    /// file paths
    pub fn add_input_str(&mut self, name: &str, content: &str) {
        assert!(
            name.starts_with("<") && name.ends_with(">"),
            "filename must be enclosed in <> brackets"
        );
        self.inputs.insert(
            name.to_owned(),
            Rc::new(Input {
                name: name.to_owned(),
                content: content.to_owned(),
                is_file: false,
            }),
        );
    }

    /// Perform all compilations
    pub fn run_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.inputs.keys().cloned().collect();
        for name in &names {
            // let messages = {
            //     let mut tuctx = self.run_one(name)?;
            //     tuctx.take_messages()
            // };
            // self.messages.insert(name.to_owned(), messages);
            let mut tuctx = self.run_one(name)?;
            let messages = tuctx.take_messages();

            self.messages.insert(name.to_owned(), messages);
        }
        Ok(())
    }

    /// Perform compilation of single translation unit
    pub fn run_one<'a>(&'a mut self, name: &str) -> Result<TUCtx<'a>> {
        // pub fn run_one<'a>(&'a mut self, name: &str) -> Result<TUCtx<'a>> {
        let mut ctx = TUCtx::from_driver(self, name);

        for pass in &ctx.driver().flags.passes {
            let f = PASS_FUNCTIONS.get(&*pass.name).unwrap();
            f(&mut ctx, &pass.args)?;
        }

        Ok(ctx)
    }

    /// Write messages to stderr
    pub fn report_messages(&self) {
        for (_name, messages) in &self.messages {
            for message in messages {
                eprintln!("{}", message);
            }
        }
    }

    /// Write output files to disk
    pub fn write_output(&self) {}

    /// Return the target we are generating code for
    pub fn target(&self) -> &dyn target::Target {
        &*self.target
    }
}

impl std::default::Default for Driver {
    fn default() -> Self {
        Driver {
            flags: Flags::new(),

            inputs: HashMap::new(),
            messages: HashMap::new(),
            target: Box::new(target::linux::X64Linux),
        }
    }
}

fn generate_clap<'a, 'b>(from_env: bool) -> clap::App<'a, 'b> {
    clap::App::new("denuocc")
        .about("denuo c compiler")
        .arg(
            clap::Arg::with_name("FILES")
                .required(from_env)
                .multiple(true),
        )
        .arg(
            clap::Arg::with_name("pass")
                .long("pass")
                .multiple(true)
                .value_delimiter(";")
                .takes_value(true),
        )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "input name not found")]
    pub fn test_driver_run_one_missing_input() {
        let mut driver = Driver::new();
        driver.run_one("missing").unwrap();
    }
}
