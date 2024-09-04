//! Expose the Varnish context (`struct vrt_ctx`) as a Rust object
use std::ffi::{c_char, c_uint, c_void, CString};
use std::ptr;

use crate::ffi;
use crate::ffi::{
    busyobj, req, sess, vrt_ctx, vsb, vsl_log, ws, VRT_fail, VSL_tag_e_SLT_Backend_health,
    VSL_tag_e_SLT_Debug, VSL_tag_e_SLT_Error, VSL_tag_e_SLT_FetchError, VSL_tag_e_SLT_VCL_Error,
    VSL_tag_e_SLT_VCL_Log, VCL_HTTP, VCL_VCL, VRT_CTX_MAGIC,
};
use crate::vcl::http::HTTP;
use crate::vcl::ws::{TestWS, WS};

/// VSL logging tag
///
/// An `enum` wrapper around [VSL tags](https://varnish-cache.org/docs/trunk/reference/vsl.html#vsl-tags).
/// Only the most current tags (for vmod writers) are mapped, and [`LogTag::Any`] will allow to
/// directly pass a native tag code (`ffi::VSL_tag_e_SLT_*`).
pub enum LogTag {
    Debug,
    Error,
    VclError,
    FetchError,
    BackendHealth,
    VclLog,
    Any(u32),
}

impl LogTag {
    pub fn into_u32(&self) -> u32 {
        match self {
            LogTag::Debug => VSL_tag_e_SLT_Debug,
            LogTag::Error => VSL_tag_e_SLT_Error,
            LogTag::VclError => VSL_tag_e_SLT_VCL_Error,
            LogTag::FetchError => VSL_tag_e_SLT_FetchError,
            LogTag::BackendHealth => VSL_tag_e_SLT_Backend_health,
            LogTag::VclLog => VSL_tag_e_SLT_VCL_Log,
            LogTag::Any(n) => *n,
        }
    }
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
    pub raw: &'a mut vrt_ctx,
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
    pub fn new(raw: *mut vrt_ctx) -> Self {
        let raw = unsafe { raw.as_mut().unwrap() };
        assert_eq!(raw.magic, VRT_CTX_MAGIC);
        let http_req = HTTP::new(raw.http_req);
        let http_req_top = HTTP::new(raw.http_req_top);
        let http_resp = HTTP::new(raw.http_resp);
        let http_bereq = HTTP::new(raw.http_bereq);
        let http_beresp = HTTP::new(raw.http_beresp);
        let ws = WS::new(raw.ws);
        Ctx {
            raw,
            http_req,
            http_req_top,
            http_resp,
            http_bereq,
            http_beresp,
            ws,
        }
    }

    /// Log an error message and fail the current VSL task.
    ///
    /// Once the control goes back to Varnish, it will see that the transaction was marked as fail
    /// and will return a synthetic error to the client.
    pub fn fail(&mut self, msg: &str) -> u8 {
        // not great, we have to copy the string to add a null character
        let c_cstring = CString::new(msg).unwrap();
        unsafe {
            VRT_fail(self.raw, c"%s".as_ptr(), c_cstring.as_ptr());
        }
        0
    }

    /// Log a message, attached to the current context
    pub fn log(&mut self, logtag: LogTag, msg: &str) {
        unsafe {
            let p = *self.raw;
            if p.vsl.is_null() {
                log(logtag, msg);
            } else {
                let t = ffi::txt {
                    b: msg.as_ptr().cast::<c_char>(),
                    e: msg.as_ptr().add(msg.len()).cast::<c_char>(),
                };
                ffi::VSLbt(p.vsl, logtag.into_u32(), t);
            }
        }
    }

    pub fn cached_req_body(&mut self) -> Result<Vec<&'a [u8]>, crate::vcl::Error> {
        unsafe extern "C" fn chunk_collector(
            priv_: *mut c_void,
            _flush: c_uint,
            ptr: *const c_void,
            len: isize,
        ) -> std::os::raw::c_int {
            let v = priv_.cast::<Vec<&[u8]>>().as_mut().unwrap();
            let buf = std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize);
            v.push(buf);
            0
        }

        let req = unsafe { self.raw.req.as_mut().ok_or("req object isn't available")? };
        unsafe {
            if req.req_body_status != ffi::BS_CACHED.as_ptr() {
                return Err("request body hasn't been previously cached".into());
            }
        }
        let mut v: Box<Vec<&'a [u8]>> = Box::default();
        let p: *mut Vec<&'a [u8]> = &mut *v;
        match unsafe {
            ffi::VRB_Iterate(
                req.wrk,
                req.vsl.as_mut_ptr(),
                req,
                Some(chunk_collector),
                p.cast::<c_void>(),
            )
        } {
            0 => Ok(*v),
            _ => Err("req.body iteration failed".into()),
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
                vclver: 0,
                msg: ptr::null::<vsb>() as *mut vsb,
                vsl: ptr::null::<vsl_log>() as *mut vsl_log,
                vcl: ptr::null::<VCL_VCL>() as VCL_VCL,
                ws: ptr::null_mut::<ws>(),
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
                vpi: ptr::null::<vsb>() as *mut ffi::wrk_vpi,
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

/// Qualify a VCL phase, mainly consumed by vmod `event` functions.
#[derive(Debug)]
pub enum Event {
    Load,
    Warm,
    Cold,
    Discard,
}

impl Event {
    /// Create a new event from a [`ffi::vcl_event_e`]. Note that it *will panic* if
    /// provided with an invalid number.
    pub fn new(event: ffi::vcl_event_e) -> Self {
        match event {
            ffi::vcl_event_e_VCL_EVENT_LOAD => Event::Load,
            ffi::vcl_event_e_VCL_EVENT_WARM => Event::Warm,
            ffi::vcl_event_e_VCL_EVENT_COLD => Event::Cold,
            ffi::vcl_event_e_VCL_EVENT_DISCARD => Event::Discard,
            _ => panic!("invalid event number"),
        }
    }
}

// TODO: avoid Stringification
pub fn log(logtag: LogTag, msg: &str) {
    unsafe {
        let c_cstring = CString::new(msg).unwrap();
        ffi::VSL(
            logtag.into_u32(),
            ffi::vxids { vxid: 0 },
            c"%s".as_ptr(),
            c_cstring.as_ptr().cast::<u8>(),
        );
    }
}
