//! # Varnish bindings
//!
//! This module provides access to various [Varnish](http://varnish-cache.org/) facilities, notably those needed to create
//! pure-rust vmods (check out examples [here](https://github.com/gquintard/varnish-rs/tree/main/examples)).
//! Note that it doesn't aim to be a 1-to-1 mirror of the C API, as Rust allows for better
//! ergonomics than what the C code can provide (notably around strings and buffer handling).
//!
//! **WARNING:** This crate is pre-1.0 and under active development so expect things to move around. There's also a lot of unsafe code and a few "shortcuts" that will be cleaned later on.
//! In short: **see this as a tech-preview, and don't run it in production.**
//!
//! # Building a VMOD
//!
//! The main idea for this crate is to make the building framework as light as possible for the
//! vmod writer, here's a checklist, but you can also just check the [source
//! code](https://github.com/gquintard/varnish-rs/tree/main/examples/vmod_example).
//!
//! The general structure of your code should look like this:
//!
//! ``` text
//! .
//! ├── build.rs
//! ├── Cargo.lock
//! ├── Cargo.toml
//! ├── README.md
//! ├── src
//! │   └── lib.rs
//! ├── tests
//! │   └── test01.vtc
//! └── vmod.vcc
//! ```
//! ## Cargo.toml
//!
//! ``` toml
//! [build-dependencies]
//! varnish = "0.0.19"
//!
//! [dependencies]
//! varnish = "0.0.19"
//! ```
//!
//! ## vmod.vcc
//!
//! You will need a [`vmod.vcc`](https://varnish-cache.org/docs/trunk/reference/vmod.html#the-vmod-vcc-file)
//! alongside your `Cargo.toml`. This file describes your vmod's API and how it'll be accessible
//! from VCL.
//!
//! The good news is that the syntax is exactly the same as for a C vmod. The bad news is that we
//! don't support all types and niceties just yet. Check out the [`vcl`] page for
//! more information.
//!
//! ``` text
//! # we need a comment at the top, possibly describing the license
//! $Module example 3 "An example vmod"
//!
//! $Function BOOL is_even(INT)
//! ```
//!
//! ## build.rs
//!
//! The `vmod.vcc`  file needs to be processed into rust-code so the module is loadable by
//! Varnish. These steps are currently done via a `python` script triggered by the `build.rs` file
//! (also alongside `Cargo.toml`).
//! The nitty-gritty details have been hidden away, and you can have a fairly simple file:
//!
//! ``` ignore
//! fn main() {
//!     varnish::generate_boilerplate().unwrap();
//! }
//! ```
//!
//! ## src/lib.rs
//!
//! Here's the actual code that you can write to implement your API. Basically, you need to
//! implement public functions that mirror what you described in `vmod.vcc`, and the first
//! argument needs to be a reference to [`vcl::Ctx`]:
//!
//! ``` ignore
//! varnish::boilerplate!();
//!
//! use varnish::vcl::Ctx;
//!
//! pub fn is_even(_: &Ctx, n: i64) -> bool {
//!     return n % 2 == 0;
//! }
//! ```
//!
//! The various type translations are described in detail in [`vcl`].

// Re-publish varnish_sys::ffi and vcl
pub use varnish_sys::{ffi, vcl};

pub mod varnishtest;
pub mod vsc;

pub use varnish_macros::vmod;

/// Automate VTC testing
///
/// Varnish provides a very handy tool for end-to-end testing:
/// [`varnishtest`](https://varnish-cache.org/docs/trunk/reference/varnishtest.html) which will
/// test various scenarios you describe in a [`VTC file`](https://varnish-cache.org/docs/trunk/reference/vtc.html):
///
/// ``` vtc
/// server s1 {
///     rxreq
///     expect req.http.even == "true"
///     txresp
/// } -start
///
/// varnish v1 -vcl+backend {
///     import example from "${vmod}";
///
///     sub vcl_recv {
///         set req.http.even = example.is_even(8);
///     }
/// } -start
///
/// client c1 {
///     txreq
///     rxresp
///     expect resp.status == 200
/// ```
///
/// Provided your VTC files are in `tests/` and have the `.vtc` extension, you can run them as part of automated testing:
///
/// ``` rust
/// varnish::run_vtc_tests!("tests/*.vtc");
/// ```
///
/// This will declare the test named `test01` and set up and run `varnishtest` alongside your unit
/// tests when you run `cargo test`.
///
/// **Important note:** you need to first build your vmod (i.e. with `cargo build`) before the tests can be run,
/// otherwise you'll get a panic.
///
/// Tests will automatically time out after 5s. To override, set `VARNISHTEST_DURATION` env var.
///
/// To debug the tests, pass `true` as the second argument:
/// ``` rust
/// varnish::run_vtc_tests!("tests/*.vtc", true);
/// ```
#[macro_export]
macro_rules! run_vtc_tests {
    ( $glob_path:expr ) => {
        $crate::run_vtc_tests!($glob_path, false);
    };
    ( $glob_path:expr, $debug:expr ) => {
        #[cfg(test)]
        #[test]
        fn run_vtc_tests() {
            if let Err(err) = $crate::varnishtest::run_all_tests(
                env!("LD_LIBRARY_PATH"),
                env!("CARGO_PKG_NAME"),
                $glob_path,
                option_env!("VARNISHTEST_DURATION").unwrap_or("5s"),
                $debug,
            ) {
                panic!("{err}");
            }
        }
    };
}
