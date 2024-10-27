#!/usr/bin/env -S cargo +nightly -Zscript
---
[dependencies.rust-script-ext]
git = "https://github.com/kurtlawrence/rust-script-ext"
rev = "2a16a21f150e7e36725195d09e4b08ebbe944548"
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
