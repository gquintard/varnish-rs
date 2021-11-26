use std::os::raw::*;
use std::ptr;
use varnish_sys::*;

pub fn empty_ctx() -> vrt_ctx {
    vrt_ctx {
        magic: VRT_CTX_MAGIC,
        syntax: 0,
        method: 0,
        handling: ptr::null::<c_uint>() as *mut c_uint,
        vclver: 0,
        msg: ptr::null::<vsb>() as *mut vsb,
        vsl: ptr::null::<vsl_log>() as *mut vsl_log,
        vcl: ptr::null::<VCL_VCL>() as VCL_VCL,
        ws: 1 as *mut ws,
        sp: ptr::null::<sess>() as *mut sess,
        req: ptr::null::<req>() as *mut req,
        http_req: ptr::null::<VCL_HTTP>() as VCL_HTTP,
        http_req_top: ptr::null::<VCL_HTTP>() as VCL_HTTP,
        http_resp: ptr::null::<VCL_HTTP>() as VCL_HTTP,
        bo: ptr::null::<VCL_HTTP>() as *mut busyobj,
        http_bereq: ptr::null::<VCL_HTTP>() as VCL_HTTP,
        http_beresp: ptr::null::<VCL_HTTP>() as VCL_HTTP,
        now: 0.0,
        specific: ptr::null::<VCL_HTTP>() as *mut c_void,
        called: ptr::null::<vsb>() as *mut c_void,
    }
}
