//! Expose the Varnish context (`struct vrt_ctx`) as a Rust object
use std::os::raw::{c_uint, c_void};

use crate::vcl::http::HTTP;
use crate::vcl::ws::{TestWS, WS};
use std::ptr;
use varnish_sys::{
    busyobj, req, sess, vrt_ctx, vsb, vsl_log, ws, VCL_HTTP, VCL_VCL, VRT_CTX_MAGIC,
    VSL_tag_e_SLT_Debug, VSL_tag_e_SLT_Error, VSL_tag_e_SLT_VCL_Error
};

// XXX: cheat: avoid dealing with too many bindgen issues and just cherry-pick VCL_RET_FAIL
const VCL_RET_FAIL: c_uint = 4;

pub enum LogTag {
    Debug,
    Error,
    VclError,
    Any(u32),
}

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
///
/// ``` rust
/// use varnish::vcl::ctx::Ctx;
///
/// fn foo(ctx: &Ctx) {
///     if let Some(ref req) = ctx.http_req {
///         for (name, value) in req {
///             println!("header {} has value {}", name, value);
///         }
///     }
/// }
/// ```
pub struct Ctx<'a> {
    pub raw: &'a varnish_sys::vrt_ctx,
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
        let raw = unsafe { raw.as_ref().unwrap() };
        assert_eq!(raw.magic, varnish_sys::VRT_CTX_MAGIC);
        Ctx {
            raw,
            http_req: HTTP::new(raw.http_req),
            http_req_top: HTTP::new(raw.http_req_top),
            http_resp: HTTP::new(raw.http_resp),
            http_bereq: HTTP::new(raw.http_bereq),
            http_beresp: HTTP::new(raw.http_beresp),
            ws: WS::new(raw.ws),
        }
    }

    pub fn fail(&mut self, msg: &str) -> u8 {
        let p = self.raw;
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
            self.log(LogTag::VclError, msg);
        }
        0
    }

    pub fn log(&mut self, logtag: LogTag, msg: &str) {
        let t = match logtag {
            LogTag::Debug => VSL_tag_e_SLT_Debug,
            LogTag::Error => VSL_tag_e_SLT_Error,
            LogTag::VclError => VSL_tag_e_SLT_VCL_Error,
            LogTag::Any(n) => n,
        };
        unsafe {
            let p = *self.raw;
            varnish_sys::VSLb_bin(
                p.vsl,
                t,
                msg.len() as i64,
                msg.as_ptr() as *const c_void,
                );
        }
    }
}

/// A struct holding both a native vrt_ctx struct, as well as the space it points to.
///
/// As the name implies, this struct mainly exist to facilitate testing and should probably not be
/// used elsewhere.
pub struct TestCtx {
    vrt_ctx: vrt_ctx,
    test_ws: TestWS,
}

impl TestCtx {
    /// Instantiate a vrt_ctx, as well as the workspace (of size `sz`) it links to.
    pub fn new(sz: usize) -> Self {
        let mut test_ctx = TestCtx {
            vrt_ctx: vrt_ctx {
                magic: VRT_CTX_MAGIC,
                syntax: 0,
                method: 0,
                handling: ptr::null::<c_uint>() as *mut c_uint,
                vclver: 0,
                msg: ptr::null::<vsb>() as *mut vsb,
                vsl: ptr::null::<vsl_log>() as *mut vsl_log,
                vcl: ptr::null::<VCL_VCL>() as VCL_VCL,
                ws: 0 as *mut ws,
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
            },
            test_ws: TestWS::new(sz),
        };
        test_ctx.vrt_ctx.ws = test_ctx.test_ws.as_ptr();
        test_ctx
    }

    pub fn ctx(&mut self) -> Ctx {
        Ctx::new(&mut self.vrt_ctx)
    }
}

#[test]
fn ctx_test() {
    let mut test_ctx = TestCtx::new(100);
    test_ctx.ctx();
}
