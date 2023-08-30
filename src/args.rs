//! Functional argument parsing.
use ::miette::*;
use std::{any::type_name, str::FromStr};

/// Get the command line [`Args`].
pub fn args() -> Args {
    let mut args = std::env::args();
    args.next(); // skip process name
    let len = args.len();
    Args {
        incoming: Box::new(args),
        seen: Vec::with_capacity(len),
        idx: 0,
        excl: vec![false; len].into_boxed_slice(),
    }
}

/// Arguments iterator.
///
/// This provides an additional utility layer on top of [`std::env::Args`].
/// It does not aim to be a fully feature argument parser,
/// [`clap`](https://docs.rs/clap/latest/clap/index.html) is great for this, at the cost of a much
/// heavier crate.
///
/// This struct is meant to provide an iterator-like interface layering on parsing and error
/// handling.
///
/// The two most common functions are [`req`](Args::req) and [`opt`](Args::opt), which will parse the current argument
/// position and advance to the next position.
///
/// To create an [`Args`], use the [`args`] function.
pub struct Args {
    /// An iterator of incoming arguments.
    incoming: Box<dyn ExactSizeIterator<Item = String>>,
    /// Already iterated arguments, in canonical order.
    seen: Vec<String>,
    /// The current argument position.
    idx: usize,
    /// Arguments to skip over when iterating.
    excl: Box<[bool]>,
}

impl Args {
    /// Parse current argument, requiring it exist, and advance the argument position.
    ///
    /// `T` should implement [`FromStr`] with `FromStr::Err` implementing [`IntoDiagnostic`].
    /// `desc` describes the argument in case of failure.
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// # use std::path::PathBuf;
    /// let mut args = Args::from(vec!["fst.txt", "24h"]);
    ///
    /// let fst = args.req::<PathBuf>("filepath").unwrap();
    /// // humantime::Duration can parse nicely
    /// let dur = args.req::<Duration>("delay length").unwrap();
    ///
    /// let err = args.req::<String>("output").unwrap_err().to_string();
    /// assert_eq!(&err, "expecting an argument at position 3");
    /// ```
    pub fn req<T>(&mut self, desc: impl AsRef<str>) -> Result<T>
    where
        T: FromStr,
        Result<T, T::Err>: IntoDiagnostic<T, T::Err>,
    {
        let desc = desc.as_ref();
        self.opt(desc)?.ok_or_else(|| {
            self.make_err(
                desc,
                format!("expecting an argument at position {}", self.idx + 1),
            )
        })
    }

    /// Parse current argument, returning `None` if it does not exist.
    /// If it does exist (and parses), advances the argument position.
    ///
    /// `T` should implement [`FromStr`] with `FromStr::Err` implementing [`IntoDiagnostic`].
    /// `desc` describes the argument in case of failure.
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// # use std::path::PathBuf;
    /// let mut args = Args::from(vec!["fst.txt"]);
    ///
    /// let fst = args.opt::<String>("filepath").unwrap();
    /// assert_eq!(fst, Some("fst.txt".to_string()));
    /// let dur = args.opt::<Duration>("delay").unwrap();
    /// assert!(dur.is_none());
    ///
    /// // parsing error
    /// let mut args = Args::from(vec!["text"]);
    /// let err = args.opt::<f64>("a number").unwrap_err();
    /// assert_eq!(&err.to_string(), "failed to parse `text` as f64");
    /// ```
    pub fn opt<T>(&mut self, desc: impl AsRef<str>) -> Result<Option<T>>
    where
        T: FromStr,
        Result<T, T::Err>: IntoDiagnostic<T, T::Err>,
    {
        let x = self
            .peek()
            .map_err(|e| self.make_err(desc.as_ref(), e.to_string()));
        if matches!(x, Ok(Some(_))) {
            self.advance_pos();
        }
        x
    }

    /// Test if there is an argument satifying the predicate.
    ///
    /// This tests from the current argument position, supplying the argument text to the predicate
    /// closure.
    ///
    /// If the argument satisfies the predicate, `true` is returned and **that argument is
    /// excluded** from future queries (including `req` and `opt`).
    ///
    /// This is useful to test for flags.
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let mut args = Args::from(vec!["fst.txt", "-c", "24h"]);
    ///
    /// let cut = args.has(|x| x == "-c" || x == "--cut");
    /// assert!(cut);
    ///
    /// // skips '-c' argument when advancing
    /// assert_eq!(&args.req::<String>("").unwrap(), "fst.txt");
    /// assert_eq!(&args.req::<String>("").unwrap(), "24h");
    /// ```
    pub fn has<P>(&mut self, mut pred: P) -> bool
    where
        P: FnMut(&str) -> bool,
    {
        let idx = self.idx;
        let mut fi = None;
        while let Some(a) = self.peek_str() {
            if pred(a) {
                fi = Some(self.idx);
                break;
            }
            self.advance_pos();
        }

        self.idx = idx; // set pos back

        match fi {
            Some(i) => {
                self.excl[i] = true;
                true
            }
            None => false,
        }
    }

    /// Assert that no more arguments should be present.
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let mut args = Args::from(vec!["fst.txt", "-c", "24h"]);
    ///
    /// args.req::<String>("").unwrap();
    ///
    /// let err = args.finish().unwrap_err().to_string();
    /// assert_eq!(&err, "unconsumed arguments provided");
    /// ```
    pub fn finish(&mut self) -> Result<()> {
        let mut x = true;
        let idx = self.idx;
        while self.peek_str().is_some() {
            x = false;
            self.advance_pos();
        }

        if x {
            return Ok(());
        }

        let (offset, src) =
            self.seen
                .iter()
                .enumerate()
                .fold((0, String::new()), |(o, s), (i, a)| {
                    let o = if i == idx { s.len() } else { o };

                    (o, s + a + " ")
                });

        Err(Error::new(diagnostic! {
            severity = Severity::Error,
            code = "Unconsumed arguments",
            labels = vec![LabeledSpan::underline(offset..src.len())],
            "unconsumed arguments provided"
        })
        .with_source_code(src))
    }

    /// Parse the current argument _without advancing the argument position._
    ///
    /// `T` should implement [`FromStr`] with `FromStr::Err` implementing [`IntoDiagnostic`].
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// # use std::str::FromStr;
    /// let mut args = Args::from(vec!["24h"]);
    ///
    /// let d = args.peek::<Duration>().unwrap().unwrap();
    /// assert_eq!(d, Duration::from_str("24h").unwrap());
    ///
    /// assert!(args.finish().is_err()); // position not advanced
    /// ```
    pub fn peek<T>(&mut self) -> Result<Option<T>>
    where
        T: FromStr,
        Result<T, T::Err>: IntoDiagnostic<T, T::Err>,
    {
        self.peek_str()
            .map(|x| {
                T::from_str(x)
                    .into_diagnostic()
                    .wrap_err_with(|| format!("failed to parse `{x}` as {}", type_name::<T>()))
            })
            .transpose()
    }

    /// Retrieve the current argument as a string _without advancing the argument position._
    ///
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let mut args = Args::from(vec!["fst.txt", "24h"]);
    ///
    /// assert_eq!(args.peek_str(), Some("fst.txt"));
    /// assert_eq!(&args.req::<String>("filepath").unwrap(), "fst.txt");
    /// assert_eq!(args.peek_str(), Some("24h"));
    /// ```
    pub fn peek_str(&mut self) -> Option<&str> {
        if self.idx >= self.seen.len() {
            self.seen.extend(self.incoming.next());
        }
        self.seen.get(self.idx).map(|x| x.as_str())
    }

    /// Retreat the argument position back one.
    ///
    /// Skips over excluded arguments.
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let mut args = Args::from(vec!["fst.txt", "-c", "24h"]);
    ///
    /// args.has(|x| x == "-c"); // exclude -c flag
    /// args.req::<String>("").ok();
    /// args.req::<String>("").ok(); // at end now
    ///
    /// args.move_back();
    /// assert_eq!(args.peek_str(), Some("24h"));
    ///
    /// args.move_back();
    /// // skips the excluded
    /// assert_eq!(args.peek_str(), Some("fst.txt"));
    /// ```
    pub fn move_back(&mut self) {
        self.idx = self.idx.saturating_sub(1);
        while self.idx > 0 && self.excl[self.idx] {
            self.idx -= 1;
        }

        if !self.excl.is_empty() && self.excl[self.idx] {
            self.advance_pos();
        }
    }

    /// Move to the front of the arguments.
    ///
    /// Skips over excluded arguments.
    ///
    /// # Example
    /// ```rust
    /// # use rust_script_ext::prelude::*;
    /// let mut args = Args::from(vec!["-c", "fst.txt", "24h"]);
    ///
    /// args.has(|x| x == "-c"); // exclude -c flag
    /// args.req::<String>("").ok();
    /// args.req::<String>("").ok(); // at end now
    ///
    /// args.move_front();
    /// assert_eq!(args.peek_str(), Some("fst.txt"));
    /// ```
    pub fn move_front(&mut self) {
        self.idx = 0;
        if !self.excl.is_empty() && self.excl[self.idx] {
            self.advance_pos();
        }
    }

    /// Advance the argument position, skipping any excluded arguments.
    fn advance_pos(&mut self) {
        self.idx += 1; // always advance one
        while self.idx < self.excl.len() && self.excl[self.idx] {
            // only advance if less than total len
            // AND the current index is flagged to exclude
            self.idx += 1;
        }
    }

    fn make_err(&self, desc: &str, msg: impl AsRef<str>) -> Error {
        let (offset, src) =
            self.seen
                .iter()
                .enumerate()
                .fold((0..0, String::new()), |(o, s), (i, a)| {
                    let o = if i == self.idx {
                        s.len()..(s.len() + a.len())
                    } else {
                        o
                    };

                    (o, s + a + " ")
                });

        let offset = if offset == (0..0) {
            src.len().saturating_sub(1)..src.len().saturating_sub(1)
        } else {
            offset
        };

        Error::new(diagnostic! {
            severity = Severity::Error,
            code = format!("Error with argument <{desc}>"),
            labels = vec![LabeledSpan::underline(offset)],
            "{}", msg.as_ref()
        })
        .with_source_code(src)
    }
}

/// Consume the _remaining_ arguments as an iterator over the raw strings.
///
/// Note that this starts from the argument position **and** skips any excluded arguments.
///
/// # Example
/// ```rust
/// # use rust_script_ext::prelude::*;
/// let mut args = Args::from(vec!["fst.txt", "-c", "24h", "output"]);
/// args.req::<String>("").unwrap();
/// args.has(|x| x== "-c");
///
/// let rem = args.into_iter().collect::<Vec<_>>();
/// assert_eq!(&rem, &[
///    "24h".to_string(),
///    "output".to_string(),
/// ]);
/// ```
impl IntoIterator for Args {
    type Item = String;
    type IntoIter = Box<dyn Iterator<Item = String>>;

    fn into_iter(self) -> Self::IntoIter {
        let Args {
            incoming,
            seen,
            idx,
            excl,
        } = self;

        Box::new(
            seen.into_iter()
                .enumerate()
                .skip(idx)
                .filter_map(move |(i, a)| (!excl[i]).then_some(a))
                .chain(incoming),
        )
    }
}

impl From<Vec<String>> for Args {
    fn from(value: Vec<String>) -> Self {
        let len = value.len();
        Self {
            incoming: Box::new(value.into_iter()),
            seen: Vec::with_capacity(len),
            idx: 0,
            excl: vec![false; len].into_boxed_slice(),
        }
    }
}

impl From<Vec<&'static str>> for Args {
    fn from(value: Vec<&'static str>) -> Self {
        let len = value.len();
        Self {
            incoming: Box::new(value.into_iter().map(Into::into)),
            seen: Vec::with_capacity(len),
            idx: 0,
            excl: vec![false; len].into_boxed_slice(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::pretty_print_err;
    use insta::assert_snapshot;

    #[test]
    fn error_printing_req() {
        let mut args = Args::from(vec!["fst.txt", "24h"]);

        assert_snapshot!(
            "parse-err",
            pretty_print_err(args.req::<Duration>("delay").unwrap_err())
        );

        let _ = args.req::<String>("filepath").unwrap();
        let _ = args.req::<String>("delay length").unwrap();
        assert_snapshot!(
            "non-existent",
            pretty_print_err(args.req::<String>("output").unwrap_err())
        );
    }

    #[test]
    fn error_printing_finish() {
        let mut args = Args::from(vec!["fst.txt", "24h"]);

        let _ = args.req::<String>("filepath").unwrap();
        assert_snapshot!(pretty_print_err(args.finish().unwrap_err()));
    }

    #[test]
    fn empty_args_no_panic() {
        let mut args = Args::from(Vec::<String>::new());

        assert!(args.req::<String>("").is_err());
        assert!(args.opt::<String>("").unwrap().is_none());
        assert!(args.peek::<String>().unwrap().is_none());
        assert!(args.peek_str().is_none());
        assert!(!args.has(|_| true));
        args.move_front();
        args.move_back();
    }
}
