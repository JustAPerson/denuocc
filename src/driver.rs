// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! Easy-to-use API for invoking the compiler
//!
//! This intended to provide the majority of the functionality needed to create
//! an executable compiler.

use std::rc::Rc;

use log::{debug, error, info};

use crate::core::{ErrorKind, Result};
use crate::front::message::Severity;
use crate::session::{Session, SessionBuilder};
use crate::tu::TranslationUnit;

/// Main interface for invoking denuocc
#[derive(Clone, Debug)]
pub struct Driver {
    /// The configuration of the compilation process
    pub session: Option<Rc<Session>>,

    /// Inputs to compile and their results
    pub tus: Vec<TranslationUnit>,
}

impl Driver {
    pub fn new() -> Self {
        Driver {
            session: None,
            tus: Vec::new(),
        }
    }

    /// Read command-line arguments from process environment
    ///
    /// The very first argument should be the binary name
    pub fn parse_cli_args_from_env(&mut self) -> Result<()> {
        let app = generate_driver_clap(true);
        self.process_clap_matches(&app.get_matches_safe()?)
    }

    /// Read command-line arguments from string
    ///
    /// Do not include the binary name as first argument
    pub fn parse_cli_args_from_str(
        &mut self,
        input: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>,
    ) -> Result<()> {
        let app = generate_driver_clap(false).setting(clap::AppSettings::NoBinaryName);
        self.process_clap_matches(&app.get_matches_from_safe(input)?)
    }

    fn process_clap_matches(&mut self, matches: &clap::ArgMatches) -> Result<()> {
        debug!("Driver::process_clap_matches() matches = {:?}", &matches);

        self.session = Some(
            SessionBuilder::new()
                .parse_cli_args_from_clap(matches)?
                .build(),
        );

        if let Some(files) = matches.values_of("FILES") {
            for file in files {
                self.add_input_file(file)?;
            }
        }

        Ok(())
    }

    /// Adds the contents of the given path to the list of input translation
    /// units
    pub fn add_input_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<()> {
        if self.tus.len() >= u16::MAX as usize {
            return Err(ErrorKind::TooManyTU.into());
        }

        let stdin_path: &std::path::Path = "-".as_ref();
        let path = path.as_ref();
        info!("Driver::add_input_file() path = {:?}", path);

        let mut tub = TranslationUnit::builder(self.session.as_ref().unwrap());
        if path == stdin_path {
            info!("Driver::add_input_file() reading from stdin");
            use std::io::Read;

            let mut content = String::new();
            std::io::stdin()
                .lock()
                .read_to_string(&mut content)
                .map_err(|e| ErrorKind::InputFileError {
                    filename: "<stdin>".to_owned(),
                    error: e,
                })?;
            tub = tub.source_string("<stdin>".to_owned(), content);
        } else {
            info!("Driver::add_input_file() reading from file");
            tub = tub.source_file(path)?;
        }
        self.tus.push(tub.build());

        Ok(())
    }

    /// Adds the given string to list of input translation units
    ///
    /// `alias` must be wrapped in angle brackets (<>) to help distinguish from
    /// file paths
    pub fn add_input_str(&mut self, alias: &str, content: &str) {
        assert!(
            alias.starts_with("<") && alias.ends_with(">"),
            "alias must be enclosed in <> brackets"
        );
        info!(
            "Driver::add_input_str() alias = {:?} content = {:?}",
            alias, content
        );

        self.tus.push(
            TranslationUnit::builder(self.session.as_ref().unwrap())
                .source_string(alias.to_owned(), content.to_owned())
                .build(),
        );
    }

    /// Perform all compilations
    pub fn run(&mut self) -> Result<()> {
        info!(
            "Driver::run_all() all names = {:?}",
            self.tus
                .iter()
                .map(|tu| tu.input().name.as_str())
                .collect::<Vec<&str>>()
        );
        for tu in &mut self.tus {
            info!("Driver::run_all() running name = {:?}", &tu.input().name);
            tu.run()?;
        }
        Ok(())
    }

    /// Write messages to stderr
    pub fn report_messages(&self) {
        for tu in &self.tus {
            for message in tu.messages() {
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
        self.tus
            .iter()
            .map(|tu| tu.messages())
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

pub fn generate_driver_clap<'a, 'b>(from_env: bool) -> clap::App<'a, 'b> {
    let mut app = clap::App::new("denuocc").about("denuo c compiler").arg(
        clap::Arg::with_name("FILES")
            .required(from_env)
            .multiple(true),
    );
    for arg in crate::core::flags::generate_clap_args() {
        app = app.arg(arg);
    }
    app
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn test_driver_nonexistent_file() {
        let mut driver = Driver::new();
        driver.parse_cli_args_from_str(&[] as &[&str]).unwrap();
        let e = driver.add_input_file("nonexistent").unwrap_err();
        assert!(if let crate::ErrorKind::InputFileError { .. } = e.kind() { true } else { false });
    }
}
