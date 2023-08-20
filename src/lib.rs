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
//! TODO
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
    pub use ::regex;
    pub use ::serde;
    pub use ::time;
}

/// Typical imports.
pub mod prelude {
    pub use super::deps;

    pub use super::args::{args, Args};

    pub use super::cmd::{
        CommandExecute, CommandString,
        Output::{self, *},
    };
    pub use crate::cmd;

    /// CSV [`Reader`](::csv::Reader) backed by a [`File`](super::fs::File).
    pub type CsvReader = ::csv::Reader<super::fs::File>;

    /// CSV [`Writer`](::csv::Writer) backed by a [`File`](super::fs::File).
    pub type CsvWriter = ::csv::Writer<super::fs::File>;

    pub use super::fs::{ls, File};
    pub use ::fastrand;
    pub use ::humantime::{parse_duration, Duration, Timestamp};
    pub use ::miette::{bail, ensure, miette, Error, IntoDiagnostic, Result, WrapErr};
    pub use ::regex::Regex;
    pub use ::serde::{de::DeserializeOwned, Deserialize, Serialize};
    pub use ::time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset, Weekday};
    pub use std::io::{Read, Write};
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
