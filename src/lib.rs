//! Opinionated set of extensions for use with
//! [`rust-script`](https://github.com/fornwall/rust-script).

//! Using `rust-script` to run Rust like a shell script is great!
//! This crate provides an opinionated set of extensions tailored towards common patterns in scripts.
//! These patterns include file reading, argument parsing, error handling.
//!
//! # Argument Parsing
//! A rudimentary argument parser is provided, simply call [`args`](args::args).
//!
//! The parsing is meant to be simple, tailored to script usage. For fully featured CLI apps,
//! consider importing [`clap`](https://docs.rs/clap/latest/clap/index.html).
//!
//! # Error Handling
//! Error handling uses the [`miette`] crate.
//! A `Result` type alias is exposed, and [`IntoDiagnostic`](prelude::IntoDiagnostic) can be used
//! to convert errors.
//!
//! ```rust
//! # use rust_script_ext::prelude::*;
//! fn foo() -> Result<String> {
//!    std::fs::read_to_string("foo.txt")
//!        .into_diagnostic()
//!        .wrap_err("failed to open 'foo.txt'")
//! }
//! ```
//!
//! # Invoking Commands
//!
//! Running commands is done through `std::process::Command`.
//! There are a few helper traits and macros to assist in:
//! 
//! 1. Building a `Command`, and
//! 2. Executing a command.
//! 
//! Building commands can leverage the [`cmd!`](crate::prelude::cmd) macro.
//! This can be used to succintly build a command with arguments.
//! 
//! ```rust
//! # use rust_script_ext::prelude::*;
//! let x = 1.0;
//! let cmd = cmd!(./my-script.sh: foo/bar, --verbose, {x + 2.14});
//! assert_eq!(&cmd.cmd_str(), "./my-script.sh foo/bar --verbose 3.14");
//! ```
//! 
//! The [`CommandExecute`](crate::prelude::CommandExecute) trait provides some methods which
//! can execute a command and automatically collect the output, along with providing verbose
//! error messages if something fails.
//! 
//! ```rust,no_run
//! # use rust_script_ext::prelude::*;
//! // Verbose means also print stdout/stderr to terminal as execution occurs
//! cmd!(ls: src).execute_str(Verbose).unwrap();
//! ```
//!
//! # Serialisation
//! [`Serialize`](::serde::Serialize), [`Deserialize`](::serde::Deserialize),
//! and [`DeserializeOwned`](::serde::de::DeserializeOwned) are all exposed.
//! Because of some path quirks with re-exported proc-macros, all derived values need to be tagged
//! with the path to the serde crate, as shown below.
//!
//! ```rust
//! # use rust_script_ext::prelude::*;
//! #[derive(Deserialize)]
//! #[serde(crate = "deps::serde")]
//! struct PizzaOrder {
//!    ham: bool,
//!    cheese: bool,
//!    pineapple: bool,
//! }
//! ```
//!
//! # Date and Time
//! Date and time is handled by exposing the [`time`](::time) crate.
//! For _duration_, [`humantime`](::humantime) is used, exposing its `Duration` directly. This is
//! done for duration parsing similar to what is experienced in unix tools.
#![warn(missing_docs)]

mod args;
mod cmd;
mod fs;

/// Exposed dependency crates.
pub mod deps {
    pub use ::csv;
    pub use ::fastrand;
    pub use ::globset;
    pub use ::humantime;
    pub use ::miette;
    pub use ::rayon;
    pub use ::regex;
    pub use ::serde;
    pub use ::time;
}

/// Typical imports.
pub mod prelude {
    pub use super::deps;

    pub use super::args::{args, Args};

    pub use super::cmd::{
        CommandExecute, CommandString, CommandBuilder,
        Output::{self, *},
    };

    /// CSV [`Reader`](::csv::Reader) backed by a [`File`](super::fs::File).
    pub type CsvReader = ::csv::Reader<super::fs::File>;

    /// CSV [`Writer`](::csv::Writer) backed by a [`File`](super::fs::File).
    pub type CsvWriter = ::csv::Writer<super::fs::File>;

    pub use super::fs::{ls, File};
    pub use ::fastrand;
    pub use ::humantime::{parse_duration, Duration, Timestamp};
    pub use ::miette::{bail, ensure, miette, Error, IntoDiagnostic, Result, WrapErr};
    pub use ::rayon;
    pub use ::regex::Regex;
    pub use ::serde::{de::DeserializeOwned, Deserialize, Serialize};
    pub use ::time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset, Weekday};
    pub use std::io::{Read, Write};

    // publically document cargs! and cmd! here

    /// Construct a `[String]` array from a list of arguments.
    /// 
    /// This macro is primarily for use with [`cmd!`](cmd), but can also be independently
    /// used, a great location is [`Command::args`](std::process::Command::args).
    /// 
    /// Arguments are delimited by commas, any text between delimiters is stringified and
    /// passed through.
    /// Arguments wrapped in braces (`{ ... }`) are treated as expressions to be evaluated.
    /// This effectively writes `{ ... }.to_string()`.
    /// 
    /// ```plaintext
    /// arg1, arg2/foo, {expr}
    /// ```
    /// 
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// 
    /// let x = "hello";
    /// let c = cargs!(foo, bar/zog, {x}, {1 + 2});
    /// assert_eq!(c, [
    ///     "foo".to_string(),
    ///     "bar/zog".to_string(),
    ///     "hello".to_string(),
    ///     "3".to_string()
    /// ]);
    /// ```
    pub use ::macros::cargs;

    /// Helper to construct a [`Command`] with arguments.
    /// 
    /// The macro uses the syntax:
    /// ```plaintext
    /// cmd: arg1, arg2
    /// ```
    /// 
    /// That is, the command path, optionally followed by a colon (`:`) followed by one or
    /// more _comma delimited_ arguments.
    /// 
    /// Note that `cmd!` defers to [`cargs!`](cargs) to parse the arguments.
    /// 
    /// The macro is powerful enough to support raw path identifiers:
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let c = cmd!(ls); // no args
    /// assert_eq!(&c.cmd_str(), "ls");
    /// 
    /// let c = cmd!(ls: foo/bar, zog);
    /// assert_eq!(&c.cmd_str(), "ls foo/bar zog");
    /// 
    /// let c = cmd!(./local-script.sh: foo/bar, zog);
    /// assert_eq!(&c.cmd_str(), "./local-script.sh foo/bar zog");
    /// ```
    /// 
    /// Literals are supported:
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let c = cmd!(ls: "foo bar", 1.23);
    /// assert_eq!(&c.cmd_str(), r#"ls "foo bar" 1.23"#);
    /// ```
    /// 
    /// Arguments wrapped in braces (`{ ... }`) are treated as expressions to be evaluated.
    /// This effectively writes `{ ... }.to_string()`.
    /// 
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let h = "hello";
    /// let c = cmd!(ls: {h}, {format!("world")});
    /// assert_eq!(&c.cmd_str(), "ls hello world");
    /// ```
    /// 
    /// [`Command`]: std::process::Command
    pub use ::macros::cmd;
}

#[cfg(test)]
fn pretty_print_err(err: miette::Error) -> String {
    use miette::*;
    let mut buf = String::new();
    GraphicalReportHandler::new_themed(GraphicalTheme::unicode_nocolor())
        .render_report(&mut buf, err.as_ref())
        .unwrap();
    buf
}
