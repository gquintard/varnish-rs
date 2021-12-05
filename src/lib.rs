//! The Varnish RunTime wrapper
//!
//! This module provides access to the public API use by VCL and vmods.

pub mod vcl {
    pub mod convert;
    pub mod ctx;
    pub mod http;
    pub mod vpriv;
    pub mod ws;
}

pub mod vmodtool;

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
