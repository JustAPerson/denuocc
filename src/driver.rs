// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Denuocc front-end

use std::collections::HashMap;
use std::rc::Rc;

use log::{debug, error, info};

pub mod error;
pub mod flags;

pub use self::error::{Error, ErrorKind, Result};
pub use self::flags::Flags;

use crate::front::input::Input;
use crate::front::message::{Message, Severity};
use crate::passes::PASS_FUNCTIONS;
use crate::tu::TUCtx;

/// Main interface for invoking denuocc
#[derive(Clone, Debug)]
pub struct Driver {
    /// A map from input names to their contents
    pub inputs: HashMap<String, Rc<Input>>,

    /// Pseudo-files that can be `#included`, searched before system paths
    pub extra_files: HashMap<String, String>,

    pub flags: Flags,

    pub messages: HashMap<String, Vec<Message>>,
}

impl Driver {
    pub fn new() -> Self {
        Driver::default()
    }

    /// Read command-line arguments from process environment
    pub fn parse_args_from_env(&mut self) -> Result<()> {
        let app = generate_clap(true);
        self.process_clap_matches(app.get_matches_safe()?)
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
        debug!("Driver::process_clap_matches() matches = {:?}", &matches);
        self.flags.process_clap_matches(&matches)?;

        if let Some(files) = matches.values_of("FILES") {
            for file in files {
                self.add_input_file(file)?;
            }
        }

        Ok(())
    }

    pub fn clear_flags(&mut self) {
        self.flags = Flags::default();
    }

    /// Adds the contents of the given path to the list of input translation
    /// units
    pub fn add_input_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let stdin_path: &std::path::Path = "-".as_ref();
        let path = path.as_ref();
        info!("Driver::add_input_file() path = {:?}", path);

        let name;
        let input;

        if path == stdin_path {
            info!("Driver::add_input_file() reading from stdin");
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
            input = Input::new(name.clone(), content, None);
        } else {
            name = path.to_string_lossy().into_owned();
            info!("Driver::add_input_file() reading from file");
            let content = std::fs::read_to_string(path).map_err(|e| ErrorKind::InputFileError {
                filename: name.to_owned(),
                error: e,
            })?;

            // make sure path we store is rooted
            let mut pathbuf = std::env::current_dir().unwrap();
            pathbuf.push(path);
            input = Input::new(name.clone(), content, Some(pathbuf));
        }

        info!("Driver::add_input_file() input = {:?}", &input);
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
        info!(
            "Driver::add_input_str() name = {:?} content = {:?}",
            name, content
        );
        self.inputs.insert(
            name.to_owned(),
            Rc::new(Input::new(name.to_owned(), content.to_owned(), None)),
        );
    }

    /// Perform all compilations
    pub fn run_all(&mut self) -> Result<()> {
        let names: Vec<String> = self.inputs.keys().cloned().collect();
        info!("Driver::run_all() all names = {:?}", &names);
        for name in names {
            info!("Driver::run_all() running name = {:?}", name);
            let mut tuctx = self.run_one(&name)?;
            let messages = tuctx.take_messages();

            self.messages.insert(name, messages);
        }
        Ok(())
    }

    /// Perform compilation of single translation unit
    pub fn run_one<'a>(&'a mut self, name: &str) -> Result<TUCtx<'a>> {
        // pub fn run_one<'a>(&'a mut self, name: &str) -> Result<TUCtx<'a>> {
        let mut ctx = TUCtx::from_driver(self, name);

        info!(
            "Driver::run_one(name = {:?}) passes to run: {:?}",
            name, &self.flags.passes
        );
        for pass in &self.flags.passes {
            let f = PASS_FUNCTIONS.get(&*pass.name).unwrap();
            debug!(
                "Driver::run_one(name = {:?}) running pass {:?}",
                name, &pass.name
            );
            f(&mut ctx, &pass.args)?;
        }

        Ok(ctx)
    }

    /// Write messages to stderr
    pub fn report_messages(&self) {
        for (_name, messages) in &self.messages {
            for message in messages {
                eprintln!("{}", message.enriched_message());
            }
        }
    }

    /// Return whether all translation units succeeded
    ///
    /// This will return `true` even if no translation units have even been run yet.
    pub fn success(&self) -> bool {
        self.count_messages(Severity::Error) == 0
    }

    /// Return the number of messages of this severity across all translation units
    pub fn count_messages(&self, severity: Severity) -> usize {
        self.messages
            .values()
            .flatten()
            .filter(|m| m.kind.get_severity() == severity)
            .count()
    }

    /// Write output files to disk
    pub fn write_output(&self) -> Result<()> {
        error!("Driver::write_output() NYI");
        Ok(())
    }
}

impl std::default::Default for Driver {
    fn default() -> Self {
        Driver {
            flags: Flags::new(),
            inputs: HashMap::new(),
            messages: HashMap::new(),
            extra_files: HashMap::new(),
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
    #[should_panic(expected = "input `missing` not found")]
    pub fn test_driver_run_one_missing_input() {
        let mut driver = Driver::new();
        driver.run_one("missing").unwrap();
    }
}
