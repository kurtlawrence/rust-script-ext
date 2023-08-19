# rust-script-ext
Opinionated set of extensions for use with
[`rust-script`](https://github.com/fornwall/rust-script).

Using `rust-script` to run Rust like a shell script is great!
This crate provides an opinionated set of extensions tailored towards common patterns in scripts.
These patterns include file reading, argument parsing, error handling.
The goal is for script writers to focus on the _business logic_, not implementing parsers, handling
errors, parsing arguments, etc.

````sh
$ cargo install rust-script
..
$ cat ./template.rs
#!/usr/bin/env -S rust-script -c
//! You might need to chmod +x your script!
//! ```cargo
//! [dependencies.rust-script-ext]
//! git = "https://github.com/kurtlawrence/rust-script-ext"
//! rev = "1735ad482fb1079fcc09afbf766a0dbb58b38bc9"
//! ```

use rust_script_ext::prelude::*;

fn main() {
    // fastrand comes from rust_script_ext::prelude::*
    let n = std::iter::repeat_with(|| fastrand::u32(1..=100))
        .take(5)
        .collect::<Vec<_>>();

    println!("Here's 5 random numbers: {n:?}");
}
$ ./template.rs
Here's 5 random numbers: [28, 97, 9, 23, 58]
````

## Template Quickstart

`template.rs` contains a simple scaffold for a `rust-script[-ext]`:

```sh
curl -L https://github.com/kurtlawrence/rust-script-ext/raw/main/template.rs -o my-script.rs
chmod +x my-script.rs
./my-script.rs
```

## What's included?

What `rust-script-ext` provides is continually evolving.
It is best to review the [API
documentation](https://kurtlawrence.github.io/rust-script-ext/rust_script_ext).
At a high level, the crate provides helpers and data structures which are common in a scripting
environment, such as easy error handling, file reading, serialisation/deserialisation, etc.
If you find something lacking, I encourage you to open a PR!

## Versioning

Versioning does not follow semver as would a normal crate.
Instead, it is best to pin a script to a revision number of the repository.
As most scripts are ephemeral, or at least not pushed to a large userbase, this encourages a simple
versioning scheme where you will be mostly guaranteed that when you run the script, it will
compile!
Leaving off the revision number should also be fairly safe (cargo should lock it to a revision),
but moving the script to another system or clearing the cache could potentially break compilation.
