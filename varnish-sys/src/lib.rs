#[allow(
    improper_ctypes,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]
#[allow(
    clippy::pedantic,
    clippy::approx_constant,
    clippy::useless_transmute,
    clippy::too_many_arguments
)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

mod extensions;
mod txt;
mod utils;
mod validate;
pub mod vcl;

pub use utils::*;
pub use validate::*;
