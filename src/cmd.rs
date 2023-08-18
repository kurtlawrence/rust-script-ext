use std::process::*;

/// Helper to construct a [`Command`] with arguments.
///
/// # Example
/// ```rust
/// # use rust_script_ext::prelude::*;
///
/// // simple invocation with simple argument
/// // arguments must be strings
/// let c = cmd!(ls "foo");
/// assert_eq!(&c.cmd_str(), "ls foo");
///
/// // arguments can be expressions
/// let c = cmd!(ls format!("file{}.csv", 1));
/// assert_eq!(&c.cmd_str(), "ls file1.csv");
///
/// // arguments with spaces are encased in quotes
/// let c = cmd!(ls "foo bar");
/// assert_eq!(&c.cmd_str(), r#"ls "foo bar""#);
///
/// // pathed programs are strings
/// let c = cmd!("./script.sh" "foo" "bar");
/// assert_eq!(&c.cmd_str(), "./script.sh foo bar");
/// ```
#[macro_export]
macro_rules! cmd {
    ($cmd:literal $($arg:expr)*) => {
        cmd!($cmd => $($arg)*)
    };
    ($cmd:tt $($arg:expr)*) => {
        cmd!(stringify!($cmd) => $($arg)*)
    };
    ($cmd:expr => $($arg:expr)*) => {{
        let cmd: &str = $cmd;
        #[allow(unused_mut)]
        let mut cmd = ::std::process::Command::new(cmd);
        $({
            let bind = $arg;
            let a: &str = bind.as_ref();
            let a = if a.contains(' ') {
                format!(r#""{a}""#)
            } else {
                a.to_string()
            };
            cmd.arg(a);
        })*
        cmd
    }};
    ($cmd:expr) => {
        cmd!($cmd =>)
    };
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
        let x = format!("{self:#?}");
        eprintln!("{x}");
        let prg = x
            .split_once("program:")
            .expect("known format")
            .1
            .split_once(",")
            .expect("known format")
            .0
            .trim()
            .trim_matches('"');

        self.get_args()
            .fold(prg.to_string(), |s, a| s + " " + &*a.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_macro_output() {
        let x = cmd!(ls).cmd_str();
        assert_eq!(&x, "ls");

        let x = cmd!(ls "foo" "bar").cmd_str();
        assert_eq!(&x, "ls foo bar");

        let x = cmd!(ls format!("foo") "bar").cmd_str();
        assert_eq!(&x, "ls foo bar");

        let x = cmd!(ls "foo bar").cmd_str();
        assert_eq!(&x, r#"ls "foo bar""#);

        let x = cmd!("./script.sh" "foo bar").cmd_str();
        assert_eq!(&x, r#"./script.sh "foo bar""#);
    }
}
