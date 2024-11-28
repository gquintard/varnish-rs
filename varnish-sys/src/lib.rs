extern crate core;

#[allow(
    improper_ctypes,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]
#[allow(
    clippy::pedantic,
    clippy::approx_constant,
    clippy::manual_c_str_literals,
    clippy::too_many_arguments,
    clippy::useless_transmute
)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

mod extensions;
mod txt;
#[cfg(not(varnishsys_6))]
mod utils;

mod validate;

pub mod vcl;

#[cfg(not(varnishsys_6))]
pub use utils::*;
pub use validate::*;
