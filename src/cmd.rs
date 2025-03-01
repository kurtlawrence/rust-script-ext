use flume::{unbounded, Sender};
use itertools::Itertools;
use miette::*;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::Path;
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

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        let (tx_so, rx_so) = unbounded();
        let (tx_se, rx_se) = unbounded();

        fn fwd(
            tx: Sender<Vec<u8>>,
            mut rdr: impl Read + Send + 'static,
            print: impl Fn(&[u8]) + Send + 'static,
        ) {
            std::thread::spawn(move || {
                let buf: &mut [u8] = &mut *Box::new([0u8; 1024 * 4]);
                while let Ok(len) = rdr.read(buf) {
                    if len == 0 {
                        break;
                    }

                    let buf = buf[..len].to_vec();
                    print(&buf);
                    let _ = tx.send(buf);
                }
            });
        }

        fwd(tx_so, stdout, move |buf| {
            if matches!(output, Output::Verbose | Output::Stdout) {
                let _ = std::io::stdout().write_all(buf);
            }
        });
        fwd(tx_se, stderr, move |buf| {
            if matches!(output, Output::Verbose | Output::Stderr) {
                let _ = std::io::stderr().write_all(buf);
            }
        });

        let xs = child
            .wait()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to execute cmd: {}", self.cmd_str()))?;

        if xs.success() {
            Ok(rx_so.into_iter().flatten().collect_vec())
        } else {
            let se = rx_se.into_iter().flatten().collect_vec();
            let se = String::from_utf8_lossy(&se).to_string();
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
/// This is useful with [`cargs!`](crate::prelude::cargs).
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

    /// Add the argument if `apply` is `true`.
    fn maybe_with_arg<S>(self, apply: bool, arg: S) -> Self
    where
        Self: Sized,
        S: AsRef<OsStr>,
    {
        if apply {
            self.with_arg(arg)
        } else {
            self
        }
    }

    /// Add the arguments if `apply` is `true`.
    fn maybe_with_args<I, S>(self, apply: bool, args: I) -> Self
    where
        Self: Sized,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        if apply {
            self.with_args(args)
        } else {
            self
        }
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

    /// Akin to [`Command::current_dir`].
    fn with_current_dir<P: AsRef<Path>>(self, path: P) -> Self;

    /// Pipe `stdout` of _this_ into `next` command.
    fn pipe(self, next: Command) -> Result<Self>
    where
        Self: Sized;

    /// Pipe `stderr` of _this_ into `next` command.
    fn pipe_stderr(self, next: Command) -> Result<Self>
    where
        Self: Sized;
}

impl CommandBuilder for Command {
    fn with_arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.arg(arg);
        self
    }

    fn with_env<K, V>(mut self, key: K, val: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env(key, val);
        self
    }

    fn with_current_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.current_dir(dir);
        self
    }

    fn pipe(mut self, mut next: Command) -> Result<Self> {
        let cmd = self
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| miette!("encountered error with command {}: {e}", self.cmd_str()))?;

        let out = cmd.stdout.expect("piped so should exist");
        let stdin = Stdio::from(out);

        next.stdin(stdin);
        Ok(next)
    }

    fn pipe_stderr(mut self, mut next: Command) -> Result<Self> {
        let cmd = self
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| miette!("encountered error with command {}: {e}", self.cmd_str()))?;

        let out = cmd.stderr.expect("piped so should exist");
        let stdin = Stdio::from(out);

        next.stdin(stdin);
        Ok(next)
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
        let x = format!("{self:#?}");
        // eprintln!("{x}");

        let prg = if cfg!(windows) {
            x.split_once(' ')
                .map(|x| x.0)
                .unwrap_or(&x)
                .trim_matches('"')
        } else {
            x.split_once("program:")
                .expect("known format")
                .1
                .split_once(',')
                .expect("known format")
                .0
                .trim()
                .trim_matches('"')
        };

        // eprintln!("{prg}");

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
                "macros",
                "src",
                "target",
                "template-cargo-script.rs",
                "template-rust-script.rs",
            ]
        );

        let x = cmd!(ls: "foo").execute_str(Verbose).unwrap_err();
        assert_snapshot!("execute-err", pretty_print_err(x));

        let x = cmd!(watcmd: "foo").execute_str(Verbose).unwrap_err();
        assert_snapshot!("unknown-cmd", pretty_print_err(x));
    }

    #[test]
    fn cmd_naming_with_env() {
        let x = cmd!(ls).with_env("YO", "zog").cmd_str();
        assert_eq!(&x, "ls");

        let x = cmd!(ls: foo, bar).with_env("YO", "zog").cmd_str();
        assert_eq!(&x, "ls foo bar");

        let x = cmd!(ls: foo, bar)
            .with_envs([("YO", "zog"), ("JO", "bar")])
            .cmd_str();
        assert_eq!(&x, "ls foo bar");
    }

    #[test]
    fn cmd_piping() {
        let x = cmd!(ls)
            .pipe(cmd!(grep: Cargo.*))
            .unwrap()
            .execute_str(Quiet)
            .unwrap();
        let mut x = x.trim().split('\n').collect::<Vec<_>>();
        x.sort();

        assert_eq!(&x, &["Cargo.lock", "Cargo.toml",]);

        let x = cmd!(ls)
            .pipe(cmd!(grep: Cargo.*))
            .unwrap()
            .pipe(cmd!(grep: toml))
            .unwrap()
            .execute_str(Quiet)
            .unwrap();
        let mut x = x.trim().split('\n').collect::<Vec<_>>();
        x.sort();

        assert_eq!(&x, &["Cargo.toml",]);

        let x = cmd!(ls: foo)
            .pipe_stderr(cmd!(grep: foo))
            .unwrap()
            .execute_str(Quiet)
            .unwrap();
        let mut x = x.trim().split('\n').collect::<Vec<_>>();
        x.sort();

        assert_eq!(&x, &["ls: cannot access 'foo': No such file or directory",]);
    }
}
