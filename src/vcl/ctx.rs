use std::os::raw::{c_uint, c_void};

use crate::vcl::http::HTTP;
use crate::vcl::ws::WS;

// XXX: cheat: avoid dealing with too many bindgen issues and just cherry-pick SLT_VCL_Error and VCL_RET_FAIL
const SLT_VCL_ERROR: c_uint = 73;
const VCL_RET_FAIL: c_uint = 4;

/// VCL context
///
/// A mutable reference to this structure is always passed to vmod functions and provides access to
/// the available HTTP objects, as well as the workspace.
///
/// This struct is a pure Rust structure, mirroring some of the C fields, so you should always use
/// the provided methods to interact with them. If they are not enough, the `raw` field is actually
/// the C original pointer that can be used to directly, and unsafely, act on the structure.
///
/// Which `http_*` are present will depend on which VCL sub routine the function is called from.
pub struct Ctx<'a> {
    pub raw: *const varnish_sys::vrt_ctx,
    pub http_req: Option<HTTP<'a>>,
    pub http_req_top: Option<HTTP<'a>>,
    pub http_resp: Option<HTTP<'a>>,
    pub http_bereq: Option<HTTP<'a>>,
    pub http_beresp: Option<HTTP<'a>>,
    pub ws: WS<'a>,
}

impl<'a> Ctx<'a> {
    /// Wrap a raw pointer into an object we can use.
    ///
    /// The pointer must be non-null, and the magic must match
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(raw: *mut varnish_sys::vrt_ctx) -> Self {
        let p = unsafe { raw.as_ref().unwrap() };
        assert_eq!(p.magic, varnish_sys::VRT_CTX_MAGIC);
        Ctx {
            raw,
            http_req: HTTP::new(p.http_req),
            http_req_top: HTTP::new(p.http_req_top),
            http_resp: HTTP::new(p.http_resp),
            http_bereq: HTTP::new(p.http_bereq),
            http_beresp: HTTP::new(p.http_beresp),
            ws: WS::new(p.ws),
        }
    }

    pub fn fail(&mut self, msg: &str) -> u8 {
        let p = unsafe { *self.raw };
        assert!(!p.handling.is_null());
        unsafe {
            if *p.handling == VCL_RET_FAIL {
                return 0;
            }
            assert!(*p.handling == 0);
            *p.handling = VCL_RET_FAIL;
        }

        if p.vsl.is_null() {
            assert!(!p.msg.is_null());
            unsafe {
                varnish_sys::VSB_bcat(p.msg, msg.as_ptr() as *const c_void, msg.len() as i64);
                varnish_sys::VSB_putc(p.msg, '\n' as i32);
            }
        } else {
            unsafe {
                varnish_sys::VSLb_bin(
                    p.vsl,
                    SLT_VCL_ERROR,
                    msg.len() as i64,
                    msg.as_ptr() as *const c_void,
                );
            }
        }
        0
    }
}
