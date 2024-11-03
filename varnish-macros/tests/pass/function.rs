#![allow(unused_variables)]

use varnish::vmod;

fn main() {}

#[vmod]
mod types {
    use std::error::Error;
    use std::ffi::CStr;
    use std::net::SocketAddr;
    use std::time::Duration;
    use varnish::ffi::VCL_STRING;
    use varnish::vcl::{CowProbe, Probe, Workspace};
    use varnish_sys::vcl::VclError;

    // void
    pub fn to_void() {
        panic!()
    }
    pub fn to_res_void_err() -> Result<(), VclError> {
        panic!()
    }
    pub fn to_res_str_err() -> Result<(), &'static str> {
        panic!()
    }
    pub fn to_res_box_err() -> Result<(), Box<dyn Error>> {
        panic!()
    }

    // bool
    pub fn type_bool(_v: bool) {}
    pub fn type_bool_dflt(#[default(true)] _v: bool) {}
    pub fn opt_bool(_v: Option<bool>) {}
    pub fn to_bool() -> bool {
        panic!()
    }
    pub fn to_res_bool() -> Result<bool, &'static str> {
        panic!()
    }

    // CStr
    pub fn type_cstr(_v: &CStr) {}
    pub fn opt_cstr(_v: Option<&CStr>) {}
    pub fn opt_cstr_req(#[required] _v: Option<&CStr>) {}
    pub fn type_cstr_dflt(#[default("baz")] _v: &CStr) {}
    pub fn type_cstr_dflt2(#[default(c"baz")] _v: &CStr) {}
    pub fn opt_cstr_dflt(#[default(c"baz")] _v: Option<&CStr>) {}
    pub fn opt_cstr_dflt2(#[default(c"baz")] _v: &CStr) {}
    // pub fn to_cstr() -> &'static CStr {
    //     panic!()
    // }
    // pub fn to_res_cstr() -> Result<&'static CStr, &'static CStr> {
    //     panic!()
    // }

    // Duration
    pub fn type_duration(_v: Duration) {}
    pub fn opt_duration(_v: Option<Duration>) {}
    pub fn to_duration() -> Duration {
        panic!()
    }
    pub fn to_res_duration() -> Result<Duration, &'static str> {
        panic!()
    }

    // f64
    pub fn type_f64(_v: f64) {}
    pub fn type_f64_dflt(#[default(42.3)] _v: f64) {}
    pub fn opt_f64(_v: Option<f64>) {}
    pub fn to_f64() -> f64 {
        panic!()
    }
    pub fn to_res_f64() -> Result<f64, &'static str> {
        panic!()
    }

    // i64
    pub fn type_i64(_v: i64) {}
    pub fn type_i64_dflt(#[default(10)] _v: i64) {}
    pub fn opt_i64(_v: Option<i64>) {}
    pub fn to_i64() -> i64 {
        panic!()
    }
    pub fn to_res_i64() -> Result<i64, &'static str> {
        panic!()
    }

    // str
    pub fn type_str(_v: &str) {}
    pub fn opt_str(_v: Option<&str>) {}
    pub fn opt_str_req(#[required] _v: Option<&str>) {}
    pub fn type_str_dflt(#[default("baz")] _v: &str) {}
    pub fn opt_str_dflt(#[default("baz")] _v: Option<&str>) {}
    pub fn to_str() -> &'static str {
        panic!()
    }
    pub fn to_res_str() -> Result<&'static str, &'static str> {
        panic!()
    }

    // String
    pub fn to_string() -> String {
        panic!()
    }
    pub fn to_opt_string() -> Option<String> {
        panic!()
    }
    pub fn to_res_string() -> Result<String, &'static str> {
        panic!()
    }
    pub fn to_res_opt_string() -> Result<Option<String>, &'static str> {
        panic!()
    }

    // Probe
    pub fn type_probe(_v: Option<Probe>) {}
    pub fn type_probe_req(#[required] _v: Option<Probe>) {}
    pub fn to_probe() -> Probe {
        panic!()
    }
    pub fn to_res_probe() -> Result<Probe, &'static str> {
        panic!()
    }

    // CowProbe<'_
    pub fn type_cow_probe(_v: Option<CowProbe<'_>>) {}
    pub fn type_cow_probe_req(#[required] _v: Option<CowProbe<'_>>) {}
    // FIXME: is it correct to return a CowProbe? If it has a lifetime, it must be tied to something else...
    pub fn to_cow_probe() -> CowProbe<'static> {
        panic!()
    }
    pub fn to_res_cow_probe() -> Result<CowProbe<'static>, &'static str> {
        panic!()
    }

    // SocketAddr
    pub fn type_ip(_v: Option<SocketAddr>) {}
    pub fn type_ip_req(#[required] _v: Option<SocketAddr>) {}
    pub fn to_ip() -> SocketAddr {
        panic!()
    }
    pub fn to_res_ip() -> Result<SocketAddr, &'static str> {
        panic!()
    }

    // VCL_STRING
    pub fn to_vcl_string() -> VCL_STRING {
        panic!()
    }
    pub fn to_res_vcl_string() -> Result<VCL_STRING, &'static str> {
        panic!()
    }

    // Mixed types
    pub fn opt_i64_opt_i64(a1: i64, a2: Option<i64>, a3: i64) -> String {
        panic!()
    }

    // Workspace
    pub fn get_ws_mut(ws: &mut Workspace) {}
    pub fn get_ws_ref(ws: &Workspace) {}
}
