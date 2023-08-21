use miette::*;
use std::ffi::OsStr;
use std::io::{self, BufRead, BufReader, Write};
use std::process::*;

/// Describes the handling of a command execution for implementors of [`CommandExecute`].
#[derive(Copy, Clone, Default)]
pub enum Output {
    /// Do not print stdout or stderr.
    Quiet,
    /// Print stdout.
    Stdout,
    /// Print stderr.
    Stderr,
    /// Print stdout and stderr. This is the default option.
    #[default]
    Verbose,
}

/// Execute a command.
///
/// This trait is intended to endow [`Command`] with `execute` and `execute_str`, handling the
/// output of execution for easy use. See the
/// [implementation on `Command`](#impl-CommandExecute-for-Command)
/// for more details.
pub trait CommandExecute {
    /// Execute and collect output into a byte buffer.
    fn execute(self, output: Output) -> Result<Vec<u8>>;

    /// Execute and collect output into string.
    fn execute_str(self, output: Output) -> Result<String>
    where
        Self: CommandString + Sized,
    {
        let cstr = self.cmd_str();
        self.execute(output).and_then(|x| {
            String::from_utf8(x)
                .into_diagnostic()
                .wrap_err("failed to encode stdout to UTF8 string")
                .wrap_err_with(|| format!("cmd str: {cstr}"))
        })
    }

    /// Run the command with no capturing IO.
    fn run(self) -> Result<()>;
}

/// Run a [`Command`] to completion and handle the output.
///
/// Execution provides a simple way to run a command to completion and capture the outputs.
/// Both stdout and stderr are captured, the `output` argument describes how they should be
/// directed to the parent stdio.
/// By default, output is [`Output::Verbose`] which prints both the stdout and stderr to the terminal.
///
/// The result of the execution is the raw stdout bytes. Use `execute_str` to try to encode this
/// into a `String`.
/// If the command exits with an error (ie [`ExitStatus::success`] is `false`), an error is
/// constructed which includes the captured stderr.
///
/// ```rust,no_run
/// # use rust_script_ext::prelude::*;
/// let ls = cmd!(ls).execute_str(Verbose).unwrap();
/// assert_eq!(&ls, "Cargo.lock
/// Cargo.toml
/// LICENSE
/// local.rs
/// README.md
/// src
/// target
/// template.rs
/// ");
/// ```
impl CommandExecute for Command {
    fn execute(mut self, output: Output) -> Result<Vec<u8>> {
        // pipe both
        let mut child = self
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to start cmd: {}", self.cmd_str()))?;

        let mut stdout = BufReader::new(child.stdout.take().expect("stdout piped"));
        let mut stderr = BufReader::new(child.stderr.take().expect("stderr piped"));

        let mut so = Vec::new();
        let mut se = String::new();
        let mut buf = Vec::new();
        let mut so_lock = io::stdout();
        let mut se_lock = io::stderr();

        loop {
            buf.clear();
            stdout
                .read_until(b'\n', &mut buf)
                .into_diagnostic()
                .wrap_err("reading stdout failed")
                .wrap_err_with(|| format!("failed to execute cmd: {}", self.cmd_str()))?;
            let no_more = buf.is_empty();

            if matches!(output, Output::Verbose | Output::Stdout) {
                so_lock.write_all(&buf).ok(); // silently fail, only a redirect
            }

            so.extend_from_slice(&buf);

            buf.clear();
            stderr
                .read_until(b'\n', &mut buf)
                .into_diagnostic()
                .wrap_err("reading stderr failed")
                .wrap_err_with(|| format!("failed to execute cmd: {}", self.cmd_str()))?;
            let no_more = no_more && buf.is_empty();

            if matches!(output, Output::Verbose | Output::Stderr) {
                se_lock.write_all(&buf).ok(); // silently fail, only a redirect
            }

            se.push_str(&String::from_utf8_lossy(&buf));

            if no_more {
                break;
            }
        }

        let xs = child
            .wait()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to execute cmd: {}", self.cmd_str()))?;

        if xs.success() {
            Ok(so)
        } else {
            Err(Error::new(diagnostic! {
                labels = vec![LabeledSpan::at(0..se.len(), "stderr")],
                "failed to execute cmd: {}", self.cmd_str(),
            })
            .with_source_code(se))
        }
    }

    /// Run a command but do not capture IO.
    ///
    /// This provides an error message displaying the command run.
    ///
    /// Use this method when the command being run uses stdio for progress bars/updates.
    fn run(mut self) -> Result<()> {
        self.status().into_diagnostic().and_then(|x| {
            if x.success() {
                Ok(())
            } else {
                Err(miette!("cmd exited with code {}: {}", x, self.cmd_str()))
            }
        })
    }
}

/// Methods on [`Command`] which take `self`.
/// 
/// This is useful with [`cargs!`](cargs).
/// 
/// # Example
/// ```rust
/// # use rust_script_ext::prelude::*;
/// cmd!(ls)
///     .with_args(cargs!(foo/bar, zog))
///     .run()
///     .ok();
/// ```
pub trait CommandBuilder {
    /// Akin to [`Command::arg`].
    fn with_arg<S: AsRef<OsStr>>(self, arg: S) -> Self;
    /// Akin to [`Command::args`].
    fn with_args<I, S>(mut self, args: I) -> Self
    where
        Self: Sized,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for a in args {
            self = self.with_arg(a);
        }
        self
    }

    /// Akin to [`Command::env`].
    fn with_env<K, V>(self, key: K, val: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>;

    /// Akin to [`Command::envs`].
    fn with_envs<I, K, V>(mut self, vars: I) -> Self
    where
        Self: Sized,
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in vars {
            self = self.with_env(k, v);
        }
        self
    }
}

impl CommandBuilder for Command {
    fn with_arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.arg(arg);
        self
    }

    fn with_env<K, V>(mut self, key: K, val: V) -> Self
        where
            K: AsRef<OsStr>,
            V: AsRef<OsStr> {
        self.env(key, val);
        self
    }
}

/// Output [`Command`] as a text string, useful for debugging.
pub trait CommandString {
    /// Format the command like a bash string.
    fn cmd_str(&self) -> String;

    /// Print the command string to stderr.
    fn debug_print(self) -> Self
    where
        Self: Sized,
    {
        eprintln!("{}", self.cmd_str());
        self
    }
}

impl CommandString for Command {
    fn cmd_str(&self) -> String {
        // note that the debug format is unstable and need careful testing/handling
        // dbg!(self);
        let x = format!("{self:?}");
        let prg = x
            .split_once(' ')
            .map(|x| x.0)
            .unwrap_or(&x)
            .trim_matches('"');
        println!("{prg}");
        // let prg = x
        //     .split_once("program:")
        //     .expect("known format")
        //     .1
        //     .split_once(",")
        //     .expect("known format")
        //     .0
        //     .trim()
        //     .trim_matches('"');

        self.get_args()
            .fold(prg.to_string(), |s, a| s + " " + &*a.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::Output::*;
    use super::*;
    use crate::prelude::*;
    use crate::pretty_print_err;
    use insta::assert_snapshot;

    #[test]
    fn cmd_macro_output() {
        let x = cmd!(ls).cmd_str();
        assert_eq!(&x, "ls");

        let x = cmd!(ls: foo, bar).cmd_str();
        assert_eq!(&x, "ls foo bar");

        let x = cmd!(ls: {format!("foo")}, bar).cmd_str();
        assert_eq!(&x, "ls foo bar");

        let x = cmd!(ls: "foo bar").cmd_str();
        assert_eq!(&x, r#"ls "foo bar""#);

        let x = cmd!(./script.sh: "foo bar").cmd_str();
        assert_eq!(&x, r#"./script.sh "foo bar""#);
    }

    #[test]
    #[cfg(unix)]
    fn cmd_execute() {
        let x = cmd!(ls).execute_str(Quiet).unwrap();
        let mut x = x.trim().split('\n').collect::<Vec<_>>();
        x.sort();

        assert_eq!(
            &x,
            &[
                "Cargo.lock",
                "Cargo.toml",
                "LICENSE",
                "README.md",
                "local.rs",
                "macros",
                "src",
                "target",
                "template.rs",
            ]
        );

        let x = cmd!(ls "foo").execute_str(Verbose).unwrap_err();
        assert_snapshot!("execute-err", pretty_print_err(x));

        let x = cmd!(watcmd "foo").execute_str(Verbose).unwrap_err();
        assert_snapshot!("unknown-cmd", pretty_print_err(x));
    }
}
