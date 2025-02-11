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
