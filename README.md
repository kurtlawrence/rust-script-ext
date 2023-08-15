# rust-script-ext
Opinionated set of extensions for use with
[`rust-script`](https://github.com/fornwall/rust-script).

Using `rust-script` to run Rust like a shell script is great!
This crate provides an opinionated set of extensions tailored towards common patterns in scripts.
These patterns include file reading, argument parsing, error handling.


## Template Quickstart

`template.rs` contains a simple scaffold for a `rust-script[-ext]`:

```sh
curl -L https://github.com/kurtlawrence/rust-script-ext/raw/main/template.rs -o my-script.rs
chmod +x my-script.rs
./my-script.rs
```
