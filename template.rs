#!/usr/bin/env -S rust-script -c
//! You might need to chmod +x your script!
//! ```cargo
//! [dependencies.rust-script-ext]
//! git = "https://github.com/kurtlawrence/rust-script-ext"
//! rev = "e914bc0a04e9216ffdfd15c47f4c39b74d98bbeb"
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
