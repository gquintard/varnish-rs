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
//! You will need both `varnish-rs` (this crate), but also
//! [`varnish-sys`](https://crates.io/crates/varnish-sys) to build the vmod as the boilerplate code
//! uses it.
//!
//! ``` toml
//! [build-dependencies]
//! varnish = "0.0.4"
//!
//! [dependencies]
//! varnish = "0.0.4"
//! varnish-sys = "0.0.4"
//! ```
//!
//! ## vmod.vcc
//!
//! You will need a [`vmod.vcc`](https://varnish-cache.org/docs/trunk/reference/vmod.html#the-vmod-vcc-file)
//! alongside your `Cargo.toml`. This file describes your vmod's API and how it'll be accessible
//! from VCL.
//!
//! The good news is that the syntax is exactly the same as for a C vmod. The bad news is that we
//! don't support all types and niceties just yet. Check out the [`crate::vcl::convert`] page for
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
//! Varnish. This steps is currently done via a `python` script triggered by the `build.rs` file
//! (also alongside `Cargo.toml`).
//! The nitty-gritty details have been hidden away and you can have a fairly simple file:
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
//! argument needs to be a reference to [`crate::vcl::ctx::Ctx`]:
//!
//! ``` ignore 
//! varnish::boilerplate!();
//!
//! use varnish::vcl::ctx::Ctx;
//!
//! pub fn is_even(_: &Ctx, n: i64) -> bool {
//!     return n % 2 == 0;
//! }
//! ```
//!
//! The various type translations are described in detail in [`crate::vcl::convert`].

use std::env;
use std::env::join_paths;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
 
pub mod vcl {
    pub mod convert;
    pub mod ctx;
    pub mod http;
    pub mod vpriv;
    pub mod ws;
}

/// Automate VTC testing
///
/// Varnish provides a very handy tool for end-to-end testing:
/// [`varnishtest`](https://varnish-cache.org/docs/trunk/reference/varnishtest.html) which will
/// test various scenarios you describe in a [`VTC
/// file`](https://varnish-cache.org/docs/trunk/reference/vtc.html), for example:
///
/// ``` vtc
///
/// server s1 {
/// 	rxreq
/// 	expect req.http.even == "true"
/// 	txresp
/// } -start
///
/// varnish v1 -vcl+backend {
/// 	import example from "${vmod}";
///
/// 	sub vcl_recv {
/// 		set req.http.even = example.is_even(8);
/// 	}
/// } -start
///
/// client c1 {
/// 	txreq
/// 	rxresp
/// 	expect resp.status == 200
/// ```
///
/// Provided your VTC files are in `tests/` and have the `.vtc` extension, you can declare these
/// them in your rust code with this macro.
///
/// ``` rust
/// varnish::vtc!(test01);
/// ```
///
/// This will declare the test named `test01` and set up and run `varnishtest` alongside your unit
/// tests when you run `cargo test`.
#[macro_export]
macro_rules! vtc {
    ( $name:ident ) => {
        #[test]
        fn $name() {
            use std::io::{self, Write};
            use std::process::Command;
            let target = if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            };
            let cmd = Command::new("varnishtest")
                .arg(concat!("tests/", stringify!($name), ".vtc"))
                .arg("-D")
                .arg(
                    String::from("vmod=")
                        + std::env::current_dir().unwrap().to_str().unwrap()
                        + "/target/"
                        + target
                        + "/lib"
                        + &std::env::var("CARGO_PKG_NAME").unwrap()
                        + ".so",
                )
                .output()
                .unwrap();
            if !cmd.status.success() {
                io::stdout().write_all(&cmd.stdout).unwrap();
                panic!(concat!("tests/", stringify!($name), ".vtc failed"));
            }
        }
    };
}

/// Convenience macro to include the generate boilerplate code.
///
/// Simply add a call to it anywhere in your code to include the code Varnish needs to load your
/// module. This requires `vmod::vmod::generate` to have been run first in `build.rs`.
#[macro_export]
macro_rules! boilerplate {
    () => {
        #[allow(non_upper_case_globals)]
        #[allow(non_camel_case_types)]
        #[allow(non_snake_case)]
        #[allow(unused_imports)]
        #[allow(dead_code)]
        mod generated {
            include!(concat!(env!("OUT_DIR"), "/generated.rs"));
        }
    };
}

/// Process the `vmod.vcc` file into the boilerplate code
///
/// This function is meant to be called from `build.rs` to translate the API described in
/// `vmod.vcc` into C-compatible code that will allow Varnish to load and use your vmod.
///
/// It does require `python3` to run as it embed a script to do the processing.
pub fn generate_boilerplate() -> Result<(), String> {
    println!("cargo:rerun-if-changed=vmod.vcc");

    let rstool_bytes = include_bytes!("vmodtool-rs.py");
    let rs_tool_path =
        join_paths([env::var("OUT_DIR").unwrap(), String::from("rstool.py")]).unwrap();
    fs::write(&rs_tool_path, &rstool_bytes)
        .unwrap_or_else(|_| panic!("couldn't write rstool.py tool in {:?}", &*rs_tool_path));

    let vmodtool_path = pkg_config::get_variable("varnishapi", "vmodtool").unwrap();
    let vmodtool_dir = (vmodtool_path.as_ref() as &Path)
        .parent()
        .expect("couldn't find the directory name containing vmodtool.py")
        .to_str()
        .unwrap()
        .to_string();

    let cmd = Command::new("python3")
        .arg(rs_tool_path)
        .arg("vmod.vcc")
        .arg("-w")
        .arg(env::var("OUT_DIR").unwrap())
        .env(
            "PYTHONPATH",
            join_paths([env::var("OUT_DIR").unwrap_or_default(), vmodtool_dir]).unwrap(),
        )
        .output()
        .expect("failed to run vmodtool");

    io::stdout().write_all(&cmd.stderr).unwrap();
    assert!(cmd.status.success());

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("generated.rs");
    fs::write(out_path, &cmd.stdout).expect("Couldn't write boilerplate!");
    Ok(())
}
