//! Opinionated set of extensions for use with
//! [`rust-script`](https://github.com/fornwall/rust-script).

//! Using `rust-script` to run Rust like a shell script is great!
//! This crate provides an opinionated set of extensions tailored towards common patterns in scripts.
//! These patterns include file reading, argument parsing, error handling.
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

mod file;

/// Exposed dependency crates.
pub mod deps {
    pub use ::csv;
    pub use ::fastrand;
    pub use ::humantime;
    pub use ::miette;
    pub use ::regex;
    pub use ::serde;
    pub use ::time;
}

/// Typical imports.
pub mod prelude {
    pub use super::deps;

    /// CSV [`Reader`](::csv::Reader) backed by a [`File`](super::file::File).
    pub type CsvReader = ::csv::Reader<super::file::File>;

    /// CSV [`Writer`](::csv::Writer) backed by a [`File`](super::file::File).
    pub type CsvWriter = ::csv::Writer<super::file::File>;

    pub use super::file::File;
    pub use ::fastrand;
    pub use ::humantime::{Duration, Timestamp, parse_duration};
    pub use ::miette::{bail, ensure, miette, Error, IntoDiagnostic, Result, WrapErr};
    pub use ::regex::Regex;
    pub use ::serde::{Serialize, Deserialize, de::DeserializeOwned};
    pub use ::time::{Month, Weekday, UtcOffset, Time, Date, OffsetDateTime, PrimitiveDateTime};
}
