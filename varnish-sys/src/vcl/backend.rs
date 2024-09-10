//! Facilities to create a VMOD backend
//!
//! [`ffi::VCL_BACKEND`] can be a bit confusing to create and manipulate, notably as they
//! involve a bunch of structures with different lifetimes and quite a lot of casting. This
//! module hopes to alleviate those issues by handling the most of them and by offering a more
//! idiomatic interface centered around vmod objects.
//!
//! Here's what's in the toolbox:
//! - [VCLBackendPtr] is just an alias for [`ffi::VCL_BACKEND`] to avoid depending on
//!   `ffi`.
//! - the [Backend] type  wraps a `Serve` struct into a C backend
//! - the [Serve] trait defines which methods to implement to act as a backend, and includes
//!   default implementations for most methods.
//! - the [Transfer] trait provides a way to generate a response body,notably handling the
//!   transfer-encoding for you.
//!
//! Note: You can check out the [example/vmod_be
//! code](https://github.com/gquintard/varnish-rs/blob/main/examples/vmod_be/src/lib.rs) for a
//! fully commented vmod.
//!
//! For a very simple example, let's build a backend that just replies with 'A' a predetermined
//! number of times.
//!
//! ```
//! use varnish::vcl::{Ctx, Backend, Serve, Transfer, VclError};
//!
//! // First we need to define a struct that implement [Transfer]:
//! struct BodyResponse {
//!     left: usize,
//! }
//!
//! impl Transfer for BodyResponse {
//!     fn read(&mut self, buf: &mut [u8]) -> Result<usize, VclError> {
//!         let mut done = 0;
//!         for p in buf {
//!              if self.left == 0 {
//!                  break;
//!              }
//!              *p = 'A' as u8;
//!              done += 1;
//!              self.left -= 1;
//!         }
//!         Ok(done)
//!     }
//! }
//!
//! // Then, we need a struct implementing `Serve` to build the headers and return a BodyResponse
//! // Here, MyBe only needs to know how many times to repeat the character
//! struct MyBe {
//!     n: usize
//! }
//!
//! impl Serve<BodyResponse> for MyBe {
//!      fn get_type(&self) -> &str { "example" }
//!
//!      fn get_headers(&self, ctx: &mut Ctx) -> Result<Option<BodyResponse>, VclError> {
//!          Ok(Some(
//!            BodyResponse { left: self.n },
//!          ))
//!      }
//! }
//!
//! // Finally, we create a `Backend` wrapping a `MyBe`, and we can ask for a pointer to give to the C
//! // layers.
//! fn some_vmod_function(ctx: &mut Ctx) {
//!     let backend = Backend::new(ctx, "name", MyBe { n: 42 }, false).expect("couldn't create the backend");
//!     let ptr = backend.vcl_ptr();
//! }
//! ```
use std::ffi::{c_char, c_int, c_void, CString};
use std::marker::PhantomData;
use std::net::{SocketAddr, TcpStream};
use std::os::unix::io::FromRawFd;
use std::ptr::{null, null_mut};
use std::time::SystemTime;

use crate::ffi::{vfp_status, VCL_BACKEND, VCL_IP};
use crate::utils::get_backend;
use crate::vcl::{Ctx, Event, IntoVCL, LogTag, VclError, VclResult, Vsb, WS};
use crate::{
    ffi, validate_director, validate_vdir, validate_vfp_ctx, validate_vfp_entry, validate_vrt_ctx,
};

/// Fat wrapper around [`VCLBackendPtr`]/[`ffi::VCL_BACKEND`].
///
/// It will handle almost all the necessary boilerplate needed to create a vmod. Most importantly, it destroys/unregisters the backend as part of it's `Drop` implementation, and
/// will convert the C methods to something more idiomatic.
///
/// Once created, a [`Backend`]'s sole purpose is to exist as a C reference for the VCL. As a
/// result, you don't want to drop it until after all the transfers are done. The most common way
/// is just to have the backend be part of a vmod object because the object won't be dropped until
/// the VCL is discarded and that can only happen once all the backend fetches are done.
#[derive(Debug)]
pub struct Backend<S: Serve<T>, T: Transfer> {
    bep: VCL_BACKEND,
    #[allow(dead_code)]
    methods: Box<ffi::vdi_methods>,
    inner: Box<S>,
    #[allow(dead_code)]
    type_: CString,
    phantom: PhantomData<T>,
}

impl<S: Serve<T>, T: Transfer> Backend<S, T> {
    /// Access the inner type wrapped by [Backend]. Note that it isn't `mut` as other threads are
    /// likely to have access to it too.
    pub fn get_inner(&self) -> &S {
        &self.inner
    }

    /// Return the C pointer wrapped by the [`Backend`]. Conventionally used by the `.backend()`
    /// methods of VCL objects.
    pub fn vcl_ptr(&self) -> VCL_BACKEND {
        self.bep
    }

    /// Create a new builder, wrapping the `inner` structure (that implements `Serve`),
    /// calling the backend `name`. If the backend has a probe attached to it, set `has_probe` to
    /// true.
    pub fn new(ctx: &mut Ctx, name: &str, be: S, has_probe: bool) -> VclResult<Self> {
        let mut inner = Box::new(be);
        let type_: CString = CString::new(inner.get_type()).map_err(|e| e.to_string())?;
        let methods = Box::new(ffi::vdi_methods {
            type_: type_.as_ptr(),
            magic: ffi::VDI_METHODS_MAGIC,
            destroy: None,
            event: Some(wrap_event::<S, T>),
            finish: Some(wrap_finish::<S, T>),
            gethdrs: Some(wrap_gethdrs::<S, T>),
            getip: Some(wrap_getip::<T>),
            healthy: has_probe.then_some(wrap_healthy::<S, T>),
            http1pipe: Some(wrap_pipe::<S, T>),
            list: Some(wrap_list::<S, T>),
            panic: Some(wrap_panic::<S, T>),
            resolve: None,
            release: None,
        });

        let bep = unsafe {
            ffi::VRT_AddDirector(
                ctx.raw,
                &*methods,
                &mut *inner as *mut S as *mut c_void,
                c"%.*s".as_ptr(),
                name.len(),
                name.as_ptr().cast::<c_char>(),
            )
        };
        if bep.0.is_null() {
            return Err(format!("VRT_AddDirector return null while creating {name}").into());
        }

        Ok(Backend {
            bep,
            type_,
            inner,
            methods,
            phantom: PhantomData,
        })
    }
}

/// The trait to implement to "be" a backend
///
/// `Serve` maps to the `vdi_methods` structure of the C api, but presented in a more
/// "rusty" form. Apart from [Serve::get_type] and [Serve::get_headers] all methods are optional.
///
/// If your backend doesn't return any content body, you can implement `Serve<()>` as `()` has a default
/// `Transfer` implementation.
pub trait Serve<T: Transfer> {
    /// What kind of backend this is, for example, pick a descriptive name, possibly linked to the
    /// vmod which creates it. Pick an ASCII string, otherwise building the [`Backend`] via
    /// [Backend::new] will fail.
    fn get_type(&self) -> &str;

    /// If the VCL pick this backend (or a director ended up choosing it), this method gets called
    /// so that the `Serve` implementer can:
    /// - inspect the request headers (`ctx.http_bereq`)
    /// - fill the response headers (`ctx.http_beresp`)
    /// - possibly return a `Transfer` object that will generate the response body
    ///
    /// If this function returns a `Ok(_)` without having set the method and protocol of
    /// `ctx.http_beresp`, we'll default to `HTTP/1.1 200 OK`
    fn get_headers(&self, _ctx: &mut Ctx) -> Result<Option<T>, VclError>;

    /// Once a backend transaction is finished, the [`Backend`] has a chance to clean up, collect
    /// data and others in the finish methods.
    fn finish(&self, _ctx: &mut Ctx) {}

    /// Is your backend healthy, and when did its health change for the last time.
    fn healthy(&self, _ctx: &mut Ctx) -> (bool, SystemTime) {
        (true, SystemTime::UNIX_EPOCH)
    }

    /// If your backend is used inside `vcl_pipe`, this method is in charge of sending the request
    /// headers that Varnish already read, and then the body. The second argument, a `TcpStream` is
    /// the raw client stream that Varnish was using (converted from a raw fd).
    ///
    /// Once done, you should return a `StreamClose` describing how/why the transaction ended.
    fn pipe(&self, ctx: &mut Ctx, _tcp_stream: TcpStream) -> StreamClose {
        ctx.log(LogTag::Error, "Backend does not support pipe");
        StreamClose::TxError
    }

    /// The method will get called when the VCL changes temperature or is discarded. It's notably a
    /// chance to start/stop probes to consume fewer resources.
    fn event(&self, _event: Event) {}

    fn panic(&self, _vsb: &mut Vsb) {}

    /// Convenience function for the implementors to call if they don't have a probe. This one is
    /// not used by Varnish directly.
    fn list_without_probe(&self, ctx: &mut Ctx, vsb: &mut Vsb, detailed: bool, json: bool) {
        if detailed {
            return;
        }
        let state = if self.healthy(ctx).0 {
            "healthy"
        } else {
            "sick"
        };
        if json {
            vsb.cat(&"[0, 0, ").unwrap();
            vsb.cat(&state).unwrap();
            vsb.cat(&"]").unwrap();
        } else {
            vsb.cat(&"0/0\t").unwrap();
            vsb.cat(&state).unwrap();
        }
    }

    /// Used to generate the output of `varnishadm backend.list`. `detailed` means the `-p`
    /// argument was passed and `json` means `-j` was passed.
    fn list(&self, ctx: &mut Ctx, vsb: &mut Vsb, detailed: bool, json: bool) {
        self.list_without_probe(ctx, vsb, detailed, json);
    }
}

/// An in-flight response body
///
/// When `Serve::get_headers()` get called, the backend [`Backend`] can return a
/// `Result<Option<Transfer>>`:
/// - `Err(_)`: something went wrong, the error will be logged and synthetic backend response will be
///   generated by Varnish
/// - `Ok(None)`: headers are set, but the response as no content body.
/// - `Ok(Some(Transfer))`: headers are set, and Varnish will use the `Transfer` object to build
///   the response body.
pub trait Transfer {
    /// The only mandatory method, it will be called repeated so that the `Transfer` object can
    /// fill `buf`. The transfer will stop if any of its calls returns an error, and it will
    /// complete successfully when `Ok(0)` is returned.
    ///
    /// `.read()` will never be called on an empty buffer, and the implementer must return the
    /// number of bytes written (which therefore must be less than the buffer size).
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, VclError>;

    /// If returning `Some(_)`, we know the size of the body generated, and it'll be used to fill the
    /// `content-length` header of the response. Otherwise, chunked encoding will be used, which is
    /// what's assumed by default.
    fn len(&self) -> Option<usize> {
        None
    }

    /// Potentially return the IP:port pair that the backend is using to transfer the body. It
    /// might not make sense for your implementation.
    fn get_ip(&self) -> Result<Option<SocketAddr>, VclError> {
        Ok(None)
    }
}

impl Transfer for () {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, VclError> {
        Ok(0)
    }
}

unsafe extern "C" fn vfp_pull<T: Transfer>(
    ctxp: *mut ffi::vfp_ctx,
    vfep: *mut ffi::vfp_entry,
    ptr: *mut c_void,
    len: *mut isize,
) -> vfp_status {
    let ctx = validate_vfp_ctx(ctxp);
    let vfe = validate_vfp_entry(vfep);

    let buf = std::slice::from_raw_parts_mut(ptr.cast::<u8>(), *len as usize);
    if buf.is_empty() {
        *len = 0;
        return vfp_status::VFP_OK;
    }

    let reader = vfe.priv1.cast::<T>().as_mut().unwrap();
    match reader.read(buf) {
        Err(e) => {
            // TODO: we should grow a VSL object
            // SAFETY: we assume ffi::VSLbt() will not store the pointer to the string's content
            let msg = ffi::txt::from_str(e.as_ref());
            ffi::VSLbt(ctx.req.as_ref().unwrap().vsl, ffi::VSL_tag_e_SLT_Error, msg);
            vfp_status::VFP_ERROR
        }
        Ok(0) => {
            *len = 0;
            vfp_status::VFP_END
        }
        Ok(l) => {
            *len = l as isize;
            vfp_status::VFP_OK
        }
    }
}

unsafe extern "C" fn wrap_event<S: Serve<T>, T: Transfer>(be: VCL_BACKEND, ev: ffi::vcl_event_e) {
    let backend: &S = get_backend(validate_director(be));
    backend.event(ev.try_into().unwrap_or_else(|e| {
        panic!("{e}: value={} for backend {}", ev.0, backend.get_type());
    }));
}

unsafe extern "C" fn wrap_list<S: Serve<T>, T: Transfer>(
    ctxp: *const ffi::vrt_ctx,
    be: VCL_BACKEND,
    vsbp: *mut ffi::vsb,
    detailed: i32,
    json: i32,
) {
    let mut ctx = Ctx::from_ptr(ctxp);
    let mut vsb = Vsb::new(vsbp);
    let backend: &S = get_backend(validate_director(be));
    backend.list(&mut ctx, &mut vsb, detailed != 0, json != 0);
}

unsafe extern "C" fn wrap_panic<S: Serve<T>, T: Transfer>(be: VCL_BACKEND, vsbp: *mut ffi::vsb) {
    let mut vsb = Vsb::new(vsbp);
    let backend: &S = get_backend(validate_director(be));
    backend.panic(&mut vsb);
}

unsafe extern "C" fn wrap_pipe<S: Serve<T>, T: Transfer>(
    ctxp: *const ffi::vrt_ctx,
    be: VCL_BACKEND,
) -> ffi::stream_close_t {
    let mut ctx = Ctx::from_ptr(ctxp);
    let req = ctx.raw.validated_req();
    let sp = req.validated_session();
    let fd = sp.fd;
    assert_ne!(fd, 0);
    let tcp_stream = TcpStream::from_raw_fd(fd);

    let backend: &S = get_backend(validate_director(be));
    sc_to_ptr(backend.pipe(&mut ctx, tcp_stream))
}

#[allow(clippy::too_many_lines)] // fixme
unsafe extern "C" fn wrap_gethdrs<S: Serve<T>, T: Transfer>(
    ctxp: *const ffi::vrt_ctx,
    be: VCL_BACKEND,
) -> c_int {
    let mut ctx = Ctx::from_ptr(ctxp);
    let be = validate_director(be);
    let backend: &S = get_backend(be);
    assert!(!be.vcl_name.is_null()); // FIXME: is this validation needed?
    validate_vdir(be); // FIXME: is this validation needed?

    match backend.get_headers(&mut ctx) {
        Ok(res) => {
            // default to HTTP/1.1 200 if the backend didn't provide anything
            let beresp = ctx.http_beresp.as_mut().unwrap();
            if beresp.status().is_none() {
                beresp.set_status(200);
            }
            if beresp.proto().is_none() {
                if let Err(e) = beresp.set_proto("HTTP/1.1") {
                    ctx.fail(format!("{}: {e}", backend.get_type()));
                    return 1;
                }
            }
            let bo = ctx.raw.bo.as_mut().unwrap();
            let Some(htc) = ffi::WS_Alloc(bo.ws.as_mut_ptr(), size_of::<ffi::http_conn>() as u32)
                .cast::<ffi::http_conn>()
                .as_mut()
            else {
                ctx.fail(format!("{}: insufficient workspace", backend.get_type()));
                return -1;
            };
            htc.magic = ffi::HTTP_CONN_MAGIC;
            htc.doclose = &ffi::SC_REM_CLOSE[0];
            htc.content_length = 0;
            match res {
                None => {
                    htc.body_status = ffi::BS_NONE.as_ptr();
                }
                Some(transfer) => {
                    match transfer.len() {
                        None => {
                            htc.body_status = ffi::BS_CHUNKED.as_ptr();
                            htc.content_length = -1;
                        }
                        Some(0) => {
                            htc.body_status = ffi::BS_NONE.as_ptr();
                        }
                        Some(l) => {
                            htc.body_status = ffi::BS_LENGTH.as_ptr();
                            htc.content_length = l as isize;
                        }
                    };
                    htc.priv_ = Box::into_raw(Box::new(transfer)).cast::<c_void>();
                    // build a vfp to wrap the Transfer object if there's something to push
                    if htc.body_status != ffi::BS_NONE.as_ptr() {
                        let Some(vfp) =
                            ffi::WS_Alloc(bo.ws.as_mut_ptr(), size_of::<ffi::vfp>() as u32)
                                .cast::<ffi::vfp>()
                                .as_mut()
                        else {
                            ctx.fail(format!("{}: insufficient workspace", backend.get_type()));
                            return -1;
                        };
                        let Ok(t) =
                            WS::new(bo.ws.as_mut_ptr()).copy_bytes_with_null(&backend.get_type())
                        else {
                            ctx.fail(format!("{}: insufficient workspace", backend.get_type()));
                            return -1;
                        };
                        vfp.name = t.as_ptr();
                        vfp.init = None;
                        vfp.pull = Some(vfp_pull::<T>);
                        vfp.fini = None;
                        vfp.priv1 = null();

                        let Some(vfe) = ffi::VFP_Push(bo.vfc, vfp).as_mut() else {
                            ctx.fail(format!("{}: couldn't insert vfp", backend.get_type()));
                            return -1;
                        };
                        // we don't need to clean vfe.priv1 at the vfp level, the backend will
                        // do it in wrap_finish
                        vfe.priv1 = htc.priv_;
                    }
                }
            }

            bo.htc = htc;
            0
        }
        Err(s) => {
            let typ = backend.get_type();
            ctx.log(LogTag::FetchError, format!("{typ}: {s}"));
            1
        }
    }
}

unsafe extern "C" fn wrap_healthy<S: Serve<T>, T: Transfer>(
    ctxp: *const ffi::vrt_ctx,
    be: VCL_BACKEND,
    changed: *mut ffi::VCL_TIME,
) -> ffi::VCL_BOOL {
    let backend: &S = get_backend(validate_director(be));

    let mut ctx = Ctx::from_ptr(ctxp);
    let (healthy, when) = backend.healthy(&mut ctx);
    if !changed.is_null() {
        *changed = when.try_into().unwrap(); // FIXME: on error?
    }
    healthy.into()
}

unsafe extern "C" fn wrap_getip<T: Transfer>(
    ctxp: *const ffi::vrt_ctx,
    _be: VCL_BACKEND,
) -> VCL_IP {
    let ctxp = validate_vrt_ctx(ctxp);
    let bo = ctxp.bo.as_ref().unwrap();
    assert_eq!(bo.magic, ffi::BUSYOBJ_MAGIC);
    let htc = bo.htc.as_ref().unwrap();
    // FIXME: document why htc does not use a different magic number
    assert_eq!(htc.magic, ffi::BUSYOBJ_MAGIC);
    let transfer = htc.priv_.cast::<T>().as_ref().unwrap();

    let mut ctx = Ctx::from_ptr(ctxp);

    transfer
        .get_ip()
        .and_then(|ip| match ip {
            Some(ip) => Ok(ip.into_vcl(&mut ctx.ws)?),
            None => Ok(VCL_IP(null())),
        })
        .unwrap_or_else(|e| {
            ctx.fail(format!("{e}"));
            VCL_IP(null())
        })
}

unsafe extern "C" fn wrap_finish<S: Serve<T>, T: Transfer>(
    ctxp: *const ffi::vrt_ctx,
    be: VCL_BACKEND,
) {
    let prev_backend: &S = get_backend(validate_director(be));

    // FIXME: shouldn't the ctx magic number be checked? If so, use validate_vrt_ctx()
    let ctx = ctxp.as_ref().unwrap();
    let bo = ctx.bo.as_mut().unwrap();

    // drop the Transfer
    // FIXME: can htc be null? We do set it to null later...
    let htc = bo.htc.as_ref().unwrap();
    if let Some(old) = htc.priv_.cast::<T>().as_mut().take() {
        drop(Box::from_raw(old));
    }
    bo.htc = null_mut();

    // FIXME?: should _prev be set to NULL?
    prev_backend.finish(&mut Ctx::from_ptr(ctx));
}

impl<S: Serve<T>, T: Transfer> Drop for Backend<S, T> {
    fn drop(&mut self) {
        unsafe {
            ffi::VRT_DelDirector(&mut self.bep);
        };
    }
}

/// Return type for [Serve::pipe]
///
/// When piping a response, the backend is in charge of closing the file descriptor (which is done
/// automatically by the rust layer), but also to provide how/why it got closed.
#[derive(Debug)]
pub enum StreamClose {
    RemClose,
    ReqClose,
    ReqHttp10,
    RxBad,
    RxBody,
    RxJunk,
    RxOverflow,
    RxTimeout,
    RxCloseIdle,
    TxPipe,
    TxError,
    TxEof,
    RespClose,
    Overload,
    PipeOverflow,
    RangeShort,
    ReqHttp20,
    VclFailure,
}

fn sc_to_ptr(sc: StreamClose) -> ffi::stream_close_t {
    unsafe {
        match sc {
            StreamClose::RemClose => ffi::SC_REM_CLOSE.as_ptr(),
            StreamClose::ReqClose => ffi::SC_REQ_CLOSE.as_ptr(),
            StreamClose::ReqHttp10 => ffi::SC_REQ_HTTP10.as_ptr(),
            StreamClose::RxBad => ffi::SC_RX_BAD.as_ptr(),
            StreamClose::RxBody => ffi::SC_RX_BODY.as_ptr(),
            StreamClose::RxJunk => ffi::SC_RX_JUNK.as_ptr(),
            StreamClose::RxOverflow => ffi::SC_RX_OVERFLOW.as_ptr(),
            StreamClose::RxTimeout => ffi::SC_RX_TIMEOUT.as_ptr(),
            StreamClose::RxCloseIdle => ffi::SC_RX_CLOSE_IDLE.as_ptr(),
            StreamClose::TxPipe => ffi::SC_TX_PIPE.as_ptr(),
            StreamClose::TxError => ffi::SC_TX_ERROR.as_ptr(),
            StreamClose::TxEof => ffi::SC_TX_EOF.as_ptr(),
            StreamClose::RespClose => ffi::SC_RESP_CLOSE.as_ptr(),
            StreamClose::Overload => ffi::SC_OVERLOAD.as_ptr(),
            StreamClose::PipeOverflow => ffi::SC_PIPE_OVERFLOW.as_ptr(),
            StreamClose::RangeShort => ffi::SC_RANGE_SHORT.as_ptr(),
            StreamClose::ReqHttp20 => ffi::SC_REQ_HTTP20.as_ptr(),
            StreamClose::VclFailure => ffi::SC_VCL_FAILURE.as_ptr(),
        }
    }
}
