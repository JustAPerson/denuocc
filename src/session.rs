// Licensed   under  the   Apache  License,   Version  2.0   <LICENSE-APACHE  or
// http://www.apache.org/licenses/LICENSE-2.0> or  the MIT  license <LICENSE-MIT
// or http://opensource.org/licenses/MIT>, at your option.  This file may not be
// copied, modified, or distributed except according to those terms.

//! State common between multiple translation units

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::core::{Flags, Result};
use crate::front::input::Input;

fn generate_session_clap<'a, 'b>() -> clap::App<'a, 'b> {
    let mut app = clap::App::new("denuocc").about("denuo c compiler");
    for arg in crate::core::flags::generate_clap_args() {
        app = app.arg(arg);
    }
    app
}

pub struct SessionBuilder {
    flags: Flags,
    extra_files: HashMap<String, String>,
}

impl SessionBuilder {
    pub fn new() -> Self {
        Self {
            flags: Flags::default(),
            extra_files: HashMap::new(),
        }
    }

    /// Read command-line arguments from string
    ///
    /// Do not include the binary name as first argument
    pub fn parse_cli_args_from_str(
        self,
        input: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>,
    ) -> Result<Self> {
        let matches = generate_session_clap()
            .setting(clap::AppSettings::NoBinaryName)
            .get_matches_from_safe(input)?;
        self.parse_cli_args_from_clap(&matches)
    }

    /// Read command-line arguments from a [`clap::ArgMatches`][clap::ArgMatches]
    pub fn parse_cli_args_from_clap(mut self, matches: &clap::ArgMatches) -> Result<Self> {
        self.flags.process_clap_matches(matches)?;
        Ok(self)
    }

    pub fn add_extra_file(mut self, alias: String, content: String) -> Self {
        self.extra_files.insert(alias.into(), content.into());
        self
    }

    pub fn add_extra_files(mut self, files: HashMap<String, String>) -> Self {
        self.extra_files.extend(files);
        self
    }

    pub fn build(self) -> Rc<Session> {
        Rc::new(Session {
            flags: self.flags,
            extra_files: self.extra_files,
        })
    }
}

/// Constant state between all translation units
#[derive(Clone, Debug)]
pub struct Session {
    extra_files: HashMap<String, String>,
    flags: Flags,
}

impl Session {
    /// Entry point to begin
    pub fn builder() -> SessionBuilder {
        SessionBuilder::new()
    }

    /// Pseudo-files that can be `#included`, searched before system paths
    pub fn extra_files(&self) -> &HashMap<String, String> {
        &self.extra_files
    }

    /// The command line arguments
    pub fn flags(&self) -> &Flags {
        &self.flags
    }

    /// Search both `<>` and `""` include paths
    ///
    /// `system` specifies whether the #include was wrapped in `<>` brackets. If
    /// true, it will search the system directories. If `system` is false, it
    /// will first attempt to use [`search_for_include_quote()`][sfiq] first,
    /// then fall back to [`search_for_include_system`][sfis].
    ///
    /// [sfiq]: Session::search_for_include_quote
    /// [sfis]: Session::search_for_include_system
    pub fn search_for_include(
        &self,
        desired_file: &str,
        including_file: Option<&Path>,
        system: bool,
    ) -> Option<Input> {
        let mut input = None;
        if !system {
            input = self.search_for_include_quote(desired_file, including_file);
        }
        if input.is_none() || system {
            input = self.search_for_include_system(desired_file);
        }
        input
    }

    /// Search only the system paths
    fn search_for_include_system(&self, desired_file: &str) -> Option<Input> {
        if let Some(content) = self.extra_files.get(desired_file) {
            return Some(Input::new(desired_file.to_owned(), content.clone(), None));
        }

        unimplemented!("searching system paths for #include"); // TODO NYI System #include paths
    }

    /// Search only the non-system paths
    ///
    /// If `including_file` is `Some`, then the directory of that file will be
    /// searched. Otherwise if it is `None`, the operating system current
    /// working directory will be searched. There is no fall back between these
    /// two in either direction.
    fn search_for_include_quote(
        &self,
        desired_file: &str,
        including_file: Option<&Path>,
    ) -> Option<Input> {
        let mut path = including_file
            .map(PathBuf::from)
            .unwrap_or(std::env::current_dir().unwrap());
        path.push(&desired_file);

        let content = std::fs::read_to_string(&path);
        if let Ok(content) = content {
            Some(Input::new(desired_file.to_owned(), content, Some(path)))
        } else {
            None
        }
    }
}
