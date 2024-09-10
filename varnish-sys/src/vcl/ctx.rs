//! Expose the Varnish context [`vrt_ctx`] as a Rust object
//!
use std::ffi::{c_int, c_uint, c_void};

use crate::ffi;
use crate::ffi::{
    vcl_event_e, vrt_ctx, VRT_fail, VSL_tag_e_SLT_Backend_health, VSL_tag_e_SLT_Debug,
    VSL_tag_e_SLT_Error, VSL_tag_e_SLT_FetchError, VSL_tag_e_SLT_VCL_Error, VSL_tag_e_SLT_VCL_Log,
    VRT_CTX_MAGIC,
};
use crate::vcl::{TestWS, VclError, HTTP, WS};

/// VSL logging tag
///
/// An `enum` wrapper around [VSL tags](https://varnish-cache.org/docs/trunk/reference/vsl.html#vsl-tags).
/// Only the most current tags (for vmod writers) are mapped, and [`LogTag::Any`] will allow to
/// directly pass a native tag code (`ffi::VSL_tag_e_SLT_*`).
#[derive(Debug, Clone, Copy)]
pub enum LogTag {
    Debug,
    Error,
    VclError,
    FetchError,
    BackendHealth,
    VclLog,
    Any(u32),
}

impl From<LogTag> for u32 {
    fn from(tag: LogTag) -> u32 {
        match tag {
            LogTag::Debug => VSL_tag_e_SLT_Debug,
            LogTag::Error => VSL_tag_e_SLT_Error,
            LogTag::VclError => VSL_tag_e_SLT_VCL_Error,
            LogTag::FetchError => VSL_tag_e_SLT_FetchError,
            LogTag::BackendHealth => VSL_tag_e_SLT_Backend_health,
            LogTag::VclLog => VSL_tag_e_SLT_VCL_Log,
            LogTag::Any(n) => n,
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
/// use varnish::vcl::Ctx;
///
/// fn foo(ctx: &Ctx) {
///     if let Some(ref req) = ctx.http_req {
///         for (name, value) in req {
///             println!("header {} has value {}", name, value);
///         }
///     }
/// }
/// ```
#[derive(Debug)]
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
    pub unsafe fn from_ptr(ptr: *const vrt_ctx) -> Self {
        Self::from_ref(ptr.cast_mut().as_mut().unwrap())
    }

    /// Instantiate from a mutable reference to a [`vrt_ctx`].
    pub fn from_ref(raw: &'a mut vrt_ctx) -> Self {
        assert_eq!(raw.magic, VRT_CTX_MAGIC);
        let http_req = HTTP::new(raw.http_req);
        let http_req_top = HTTP::new(raw.http_req_top);
        let http_resp = HTTP::new(raw.http_resp);
        let http_bereq = HTTP::new(raw.http_bereq);
        let http_beresp = HTTP::new(raw.http_beresp);
        let ws = WS::new(raw.ws);
        Self {
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
    pub fn fail(&mut self, msg: impl AsRef<str>) {
        let msg = msg.as_ref();
        unsafe {
            VRT_fail(self.raw, c"%.*s".as_ptr(), msg.len(), msg.as_ptr());
        }
    }

    /// Log a message, attached to the current context
    pub fn log(&mut self, logtag: LogTag, msg: impl AsRef<str>) {
        unsafe {
            let vsl = self.raw.vsl;
            if vsl.is_null() {
                log(logtag, msg);
            } else {
                let msg = ffi::txt::from_str(msg.as_ref());
                ffi::VSLbt(vsl, logtag.into(), msg);
            }
        }
    }

    pub fn cached_req_body(&mut self) -> Result<Vec<&'a [u8]>, VclError> {
        unsafe extern "C" fn chunk_collector(
            priv_: *mut c_void,
            _flush: c_uint,
            ptr: *const c_void,
            len: isize,
        ) -> c_int {
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

/// A struct holding both a native [`vrt_ctx`] struct and the space it points to.
///
/// As the name implies, this struct mainly exist to facilitate testing and should probably not be
/// used elsewhere.
#[derive(Debug)]
pub struct TestCtx {
    vrt_ctx: vrt_ctx,
    test_ws: TestWS,
}

impl TestCtx {
    /// Instantiate a [`vrt_ctx`], as well as the workspace (of size `sz`) it links to.
    pub fn new(sz: usize) -> Self {
        let mut test_ctx = TestCtx {
            vrt_ctx: vrt_ctx {
                magic: VRT_CTX_MAGIC,
                ..vrt_ctx::default()
            },
            test_ws: TestWS::new(sz),
        };
        test_ctx.vrt_ctx.ws = test_ctx.test_ws.as_ptr();
        test_ctx
    }

    pub fn ctx(&mut self) -> Ctx {
        Ctx::from_ref(&mut self.vrt_ctx)
    }
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
    /// Create a new event from a [`ffi::vcl_event_e`].
    ///
    /// # Panics
    ///
    /// Panics if provided with an unrecognized number.
    pub fn from_raw(event: vcl_event_e) -> Self {
        event
            .try_into()
            .unwrap_or_else(|e| panic!("{e}: vcl_event_e == {}", event.0))
    }
}

impl TryFrom<vcl_event_e> for Event {
    type Error = &'static str;

    fn try_from(event: vcl_event_e) -> Result<Self, Self::Error> {
        Ok(match event {
            vcl_event_e::VCL_EVENT_LOAD => Self::Load,
            vcl_event_e::VCL_EVENT_WARM => Self::Warm,
            vcl_event_e::VCL_EVENT_COLD => Self::Cold,
            vcl_event_e::VCL_EVENT_DISCARD => Self::Discard,
            _ => Err("unrecognized event value")?,
        })
    }
}

pub fn log(logtag: LogTag, msg: impl AsRef<str>) {
    let msg = msg.as_ref();
    unsafe {
        ffi::VSL(
            logtag.into(),
            ffi::vxids { vxid: 0 },
            c"%.*s".as_ptr(),
            msg.len(),
            msg.as_ptr(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctx_test() {
        let mut test_ctx = TestCtx::new(100);
        test_ctx.ctx();
    }
}
