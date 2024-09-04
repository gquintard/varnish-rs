#![allow(improper_ctypes)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::approx_constant)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::useless_transmute)]
#![allow(clippy::too_many_arguments)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
