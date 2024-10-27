#![allow(unused_variables)]

use varnish::vmod;

fn main() {}

// FIXME: Some of the Result<T, E> return types are not implemented yet

#[vmod]
mod vcl_returns {
    use varnish::ffi::{
        VCL_ACL, VCL_BACKEND, VCL_BLOB, VCL_BODY, VCL_BOOL, VCL_BYTES, VCL_DURATION, VCL_ENUM,
        VCL_HEADER, VCL_HTTP, VCL_INSTANCE, VCL_INT, VCL_IP, VCL_PROBE, VCL_REAL, VCL_REGEX,
        VCL_STEVEDORE, VCL_STRANDS, VCL_STRING, VCL_SUB, VCL_TIME, VCL_VCL,
    };

    pub fn val_acl() -> VCL_ACL {
        panic!()
    }
    pub fn res_acl() -> Result<VCL_ACL, &'static str> {
        panic!()
    }
    pub fn val_backend() -> VCL_BACKEND {
        panic!()
    }
    pub fn res_backend() -> Result<VCL_BACKEND, &'static str> {
        panic!()
    }
    pub fn val_blob() -> VCL_BLOB {
        panic!()
    }
    pub fn res_blob() -> Result<VCL_BLOB, &'static str> {
        panic!()
    }
    pub fn val_body() -> VCL_BODY {
        panic!()
    }
    pub fn res_body() -> Result<VCL_BODY, &'static str> {
        panic!()
    }
    pub fn val_bool() -> VCL_BOOL {
        panic!()
    }
    pub fn res_bool() -> Result<VCL_BOOL, &'static str> {
        panic!()
    }
    pub fn val_bytes() -> VCL_BYTES {
        panic!()
    }
    pub fn res_bytes() -> Result<VCL_BYTES, &'static str> {
        panic!()
    }
    pub fn val_duration() -> VCL_DURATION {
        panic!()
    }
    pub fn res_duration() -> Result<VCL_DURATION, &'static str> {
        panic!()
    }
    pub fn val_enum() -> VCL_ENUM {
        panic!()
    }
    pub fn res_enum() -> Result<VCL_ENUM, &'static str> {
        panic!()
    }
    pub fn val_header() -> VCL_HEADER {
        panic!()
    }
    pub fn res_header() -> Result<VCL_HEADER, &'static str> {
        panic!()
    }
    pub fn val_http() -> VCL_HTTP {
        panic!()
    }
    pub fn res_http() -> Result<VCL_HTTP, &'static str> {
        panic!()
    }
    pub fn val_instance() -> VCL_INSTANCE {
        panic!()
    }
    // pub fn res_instance() -> Result<VCL_INSTANCE, &'static str> {
    //     panic!()
    // }
    pub fn val_int() -> VCL_INT {
        panic!()
    }
    pub fn res_int() -> Result<VCL_INT, &'static str> {
        panic!()
    }
    pub fn val_ip() -> VCL_IP {
        panic!()
    }
    pub fn res_ip() -> Result<VCL_IP, &'static str> {
        panic!()
    }
    pub fn val_probe() -> VCL_PROBE {
        panic!()
    }
    pub fn res_probe() -> Result<VCL_PROBE, &'static str> {
        panic!()
    }
    pub fn val_real() -> VCL_REAL {
        panic!()
    }
    pub fn res_real() -> Result<VCL_REAL, &'static str> {
        panic!()
    }
    pub fn val_regex() -> VCL_REGEX {
        panic!()
    }
    pub fn res_regex() -> Result<VCL_REGEX, &'static str> {
        panic!()
    }
    pub fn val_stevedore() -> VCL_STEVEDORE {
        panic!()
    }
    pub fn res_stevedore() -> Result<VCL_STEVEDORE, &'static str> {
        panic!()
    }
    pub fn val_strands() -> VCL_STRANDS {
        panic!()
    }
    pub fn res_strands() -> Result<VCL_STRANDS, &'static str> {
        panic!()
    }
    pub fn val_string() -> VCL_STRING {
        panic!()
    }
    pub fn res_string() -> Result<VCL_STRING, &'static str> {
        panic!()
    }
    pub fn val_sub() -> VCL_SUB {
        panic!()
    }
    pub fn res_sub() -> Result<VCL_SUB, &'static str> {
        panic!()
    }
    pub fn val_time() -> VCL_TIME {
        panic!()
    }
    pub fn res_time() -> Result<VCL_TIME, &'static str> {
        panic!()
    }
    pub fn val_vcl() -> VCL_VCL {
        panic!()
    }
    pub fn res_vcl() -> Result<VCL_VCL, &'static str> {
        panic!()
    }
}
