#!/usr/bin/env -S rust-script -c
//! You might need to chmod +x your script!
//! ```cargo
//! [dependencies.rust-script-ext]
//! git = "https://github.com/kurtlawrence/rust-script-ext"
//! rev = "eebd6056481617baba58840bfeb03c23b68b70f7"
//! ```

use rust_script_ext::prelude::*;

fn main() -> Result<()> {
    // fastrand comes from rust_script_ext::prelude::*
    let n = std::iter::repeat_with(|| fastrand::u32(1..=100))
        .take(5)
        .collect::<Vec<_>>();

    println!("Here's 5 random numbers: {n:?}");
    Ok(())
}
