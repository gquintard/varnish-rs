extern crate core;

#[allow(
    improper_ctypes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_qualifications
)]
#[allow(
    clippy::approx_constant,
    clippy::manual_c_str_literals,
    clippy::pedantic,
    clippy::ptr_offset_with_cast,
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
