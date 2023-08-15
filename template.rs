#!/usr/bin/env rust-script
//! Don't forget chmod +x your script!
//! ```cargo
//! [dependencies]
//! rust-script-ext = { git = "https://github.com/kurtlawrence/rust-script-ext" }
//! ```

use rust_script_ext::prelude::*;

fn main() {
    let n = std::iter::repeat_with(|| fastrand::u32(1..=100))
        .take(5)
        .collect::<Vec<_>>();

    println!("Here's 5 random numbers: {n:?}");
}
