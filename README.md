# rust-script-ext
Opinionated set of extensions for use with
[`rust-script`](https://github.com/fornwall/rust-script) or
[`cargo script`](https://github.com/rust-lang/rfcs/pull/3503).

Using `rust-script` to run Rust like a shell script is great!
This crate provides an opinionated set of extensions tailored towards common patterns in scripts.
These patterns include file reading, argument parsing, error handling.
The goal is for script writers to focus on the _business logic_, not implementing parsers, handling
errors, parsing arguments, etc.

## Using `rust-script`
````sh
$ cargo install rust-script
..
$ cat ./template-rust-script.rs
#!/usr/bin/env -S rust-script -c
//! You might need to chmod +x your script!
//! ```cargo
//! [dependencies.rust-script-ext]
//! git = "https://github.com/kurtlawrence/rust-script-ext"
//! rev = "fb0c2c888881b1e0821d21a5c9c87a7f7731b622"
//! ```
// See <https://kurtlawrence.github.io/rust-script-ext/rust_script_ext/> for documentation
use rust_script_ext::prelude::*;

fn main() -> Result<()> {
    // fastrand comes from rust_script_ext::prelude::*
    let n = std::iter::repeat_with(|| fastrand::u32(1..=100))
        .take(5)
        .collect::<Vec<_>>();

    println!("Here's 5 random numbers: {n:?}");
    Ok(())
}
$ ./template-rust-script.rs
Here's 5 random numbers: [28, 97, 9, 23, 58]
````

## Using `cargo script`
````sh
$ cat ./template-cargo-script.rs
#!/usr/bin/env -S cargo +nightly -Zscript
---
[dependencies.rust-script-ext]
git = "https://github.com/kurtlawrence/rust-script-ext"
rev = "fb0c2c888881b1e0821d21a5c9c87a7f7731b622"
---
// You might need to chmod +x your script!
// See <https://kurtlawrence.github.io/rust-script-ext/rust_script_ext/> for documentation
use rust_script_ext::prelude::*;

fn main() -> Result<()> {
    // fastrand comes from rust_script_ext::prelude::*
    let n = std::iter::repeat_with(|| fastrand::u32(1..=100))
        .take(5)
        .collect::<Vec<_>>();

    println!("Here's 5 random numbers: {n:?}");
    Ok(())
}
$ ./template-cargo-script.rs
Here's 5 random numbers: [91, 65, 32, 75, 39]
````

## Template Quickstart

`template-rust-script.rs` contains a simple scaffold for use with `rust-script`:

```sh
curl -L https://github.com/kurtlawrence/rust-script-ext/raw/main/template-rust-script.rs -o my-script.rs
chmod +x my-script.rs
./my-script.rs
```

`template-cargo-script.rs` contains a simple scaffold for use with `cargo-script`:

```sh
curl -L https://github.com/kurtlawrence/rust-script-ext/raw/main/template-cargo-script.rs -o my-script.rs
chmod +x my-script.rs
./my-script.rs
```

> `cargo script` does not (currently) set up an isolated environment when running the script, which
> can cause errors if the script lives _within a Rust crate_.
> I recommend using `rust-script` instead.

## What's included?

What `rust-script-ext` provides is continually evolving.
It is best to review the [API
documentation](https://kurtlawrence.github.io/rust-script-ext/rust_script_ext).
At a high level, the crate provides helpers and data structures which are common in a scripting
environment, such as easy error handling, file reading, serialisation/deserialisation, etc.
If you find something lacking, I encourage you to open a PR!

## Language Server Support

> Note: only tested with `rust-script`.

[`rscls`](https://github.com/MiSawa/rscls/) works as a middle-man LSP between rust-analyzer and
rust-script.
Below are instructions for getting LSP support using Neovim.

First, ensure you have `rscls` installed.
```sh
cargo install rscls
```

Next, add the following to your configuration. Note this is for Neovim LSP config.

```lua
-- Rust script LSP support through rscls
local lsp_configs = require 'lspconfig.configs'
if not lsp_configs.rscls then
	lsp_configs.rscls = {
		default_config = {
			cmd = { 'rscls' },
		    filetypes = { 'rustscript' },
		    root_dir = function(fname)
		        return require'lspconfig'.util.path.dirname(fname)
		    end,
		},
	}
end
require 'lspconfig'.rscls.setup {}
```

Then, when you are wanting LSP support in a Rust file, set the file type to `rustscript` with a
command.

```vim
:set filetype=rustscript
```

I generally do not bother with LSP support for small scripts, but it comes in handy for more
complex ones!
_Note that it will take a little bit to spool up the server and compile the script._


## Versioning

Versioning does not follow semver as would a normal crate.
Instead, it is best to pin a script to a revision number of the repository.
As most scripts are ephemeral, or at least not pushed to a large userbase, this encourages a simple
versioning scheme where you will be mostly guaranteed that when you run the script, it will
compile!
Leaving off the revision number should also be fairly safe (cargo should lock it to a revision),
but moving the script to another system or clearing the cache could potentially break compilation.
