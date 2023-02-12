//! Facilities to create a VMOD backend
//!
//! [`varnish_sys::VCL_BACKEND`] can be a bit confusing to create and manipulate, notably as they
//! involves a bunch of structures with different lifetimes and quite a lot of casting. This
//! modules hopes to alleviate those issues by handling the most of them and by offering a more
//! idiomatic interface centered around vmod objects.
use std::ffi::CString;
use std::ffi::c_char;
use std::ptr;
use std::time::SystemTime;
use std::net::{TcpStream, SocketAddr};
use std::os::unix::io::FromRawFd;

use std::os::raw::c_void;

use crate::vcl::Result;
use crate::vcl::convert::IntoVCL;
use crate::vcl::ctx::{ Ctx, Event, LogTag };
use crate::vcl::vsb::Vsb;
use crate::vcl::ws::WS;

/// Alias for [`varnish_sys::VCL_BACKEND`]
pub type VCLBackendPtr = varnish_sys::VCL_BACKEND;

/// Fat wrapper around [`VCLBackendPtr`]/[`varnish_sys::VCL_BACKEND`].
///
/// It will handle almost all the necessary boilerplate needed to create a vmod. Most importantly, it destrosy/deregisters the backend as part of it's `Drop` implementation, and
/// will convert the C methods to somehting more idomatic. Once created, a [`Backend`]'s sole purpose
/// is to exist as a C reference for the VCL. Hence, all its fields are private, and the only method it provides returns a
/// [`VCLBackendPtr`] pointer pointing to the wrapped C structure.
///
/// You can use a [`BackendBuilder`] to initiate and register a backend based on any object
/// implementing the `Serve` trait.
pub struct Backend<S: Serve<T>, T: Transfer> {
    bep: *const varnish_sys::director,
    #[allow(dead_code)]
    methods: Box<varnish_sys::vdi_methods>,
    inner: Box<S>,
    #[allow(dead_code)]
    type_: CString,
    phantom: std::marker::PhantomData<T>,
}

impl<S: Serve<T>, T: Transfer> Backend<S, T> {
    /// Return the C pointer wrapped by the [`Backend`]. Conventionally used by the `.backend()`
    /// methods of VCL objects.
    pub fn vcl_ptr(&self) -> *const varnish_sys::director {
        self.bep
    }

    /// Create a new builder, wrapping the `inner` structure (that implements `Serve`), planning to
    /// call the backend `name`.
    pub fn new(ctx: &mut Ctx, name: &str, be: S) -> Result<Self> {
        let mut inner = Box::new(be);
        let cstring_name: CString = CString::new(inner.get_type()).map_err(|e| e.to_string())?;
        let type_: CString = CString::new(name).map_err(|e| e.to_string())?;
        let methods = Box::new(varnish_sys::vdi_methods {
                type_: type_.as_ptr(),
                magic: varnish_sys::VDI_METHODS_MAGIC,
                destroy: None,
                event: Some(wrap_event::<S, T>),
                finish: Some(wrap_finish::<S, T>),
                gethdrs: Some(wrap_gethdrs::<S, T>),
                getip: Some(wrap_getip::<T>),
                healthy: Some(wrap_healthy::<S, T>),
                http1pipe: Some(wrap_pipe::<S, T>),
                list: Some(wrap_list::<S, T>),
                panic: Some(wrap_panic::<S, T>),
                resolve: None,
            });

        let bep = unsafe {
            varnish_sys::VRT_AddDirector(
                ctx.raw,
                &*methods,
                &mut *inner as *mut S as *mut std::ffi::c_void,
                "%s".as_ptr() as *const c_char,
                cstring_name.as_ptr() as *const c_char,
            )
        };
        if bep.is_null() {
            return Err(format!("VRT_AddDirector return null while creating {name}").into());
        }

        Ok(Backend {
            bep,
            type_,
            inner,
            methods,
            phantom: std::marker::PhantomData,
        })
    }
}

/// `Serve` as a trait maps to the `vdi_methods` structure of the C api, but presented in a more
/// "rusty" form. Apart from `get_type`, all methods are optional, as a [`BackendBuilder`] can choose
/// to ignore the unimplemented ones.
pub trait Serve<T: Transfer> {
    /// What kind of backend this is, for example, pick a descriptive name, possibly linked to the
    /// vmod which creates it. Pick an ASCII string, otherwise building the [`Backend`] via a
    /// [`BackendBuilder`] will fail.
   fn get_type(&self) -> String;

   /// If the VCL pick this backend (or a director ended up choosing it), this method gets called
   /// so that the `Serve` implementer can:
   /// - inspect the request headers (`ctx.http_bereq`)
   /// - fill the response headers (`ctx.http_beresp`)
   /// - possibly return a `Transfer` object that will generate the response body
   ///
   /// If this function returns a `Ok(_)` without having set the method and protocol of
   /// `ctx.http_beresp`, a [`Backend`] created with [`BackendBuilder`] will just default to
   /// `200` and `HTTP/1.1`.
    fn get_headers(&self, _ctx: &mut Ctx) -> Result<Option<T>>;

    /// Once a backend transaction is finished, the [`Backend`] has a chance to clean up, collect
    /// data and others in the finish methods.
    fn finish(&self, _ctx: &mut Ctx) {}

    /// TODO
    fn healthy(&self, _ctx: &mut Ctx) -> (bool, SystemTime) { (true, SystemTime::UNIX_EPOCH) }

    fn pipe(&self, ctx: &mut Ctx, _tcp_stream: TcpStream) -> StreamClose {
        ctx.log(LogTag::Error, "Backend does not support pipe");
        StreamClose::TxError
    }

    fn event(&self, _event: Event) {}

    fn panic(&self, _vsb: &mut Vsb) {}

    fn list(&self, _ctx: &mut Ctx, _vsb: &mut Vsb, _detailed: bool, _json: bool) {}
}

/// When `Serve::get_headers()` get called, the backend [`Backend`] can return a
/// `Result<Option<Transfer>>`:
/// - Err(_): something went wrong, the error will be logged and synthetic backend response will be
/// generated by Varnish
/// - Ok(None): headers are set, but the response as no body.
/// - Ok(Some(transfer)): headers are set, and Varnish can `transfer`'s method to get extra
/// innerrmation about the transfer and create the body.
pub trait Transfer {
    /// The only mandatory method, it will be called repeated so that the `Transfer` object can
    /// fill `buf`. The transfer will stop if any of its calls returns an error, and it will
    /// complete successfully when `Ok(0)` is returned.
    ///
    /// `.read()` will never be called on an empty buffer, and the implementer must return the
    /// number of bytes written (which therefore must be less than the buffer size).
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// If returning `Some(_)`, we know the size of the body generated, and it'll be used to fill the
    /// `content-length` header ofthe response. Otherwise chunked encoding will be used, which is
    /// what's assumed by default.
    fn len(&self) -> Option<usize> {None}

    /// TODO
    fn get_ip(&self) -> Result<Option<SocketAddr>> {Ok(None)}
}

impl Transfer for () {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize> {Ok(0)}
}

unsafe extern "C" fn vfp_pull<T: Transfer>(
    ctxp: *mut varnish_sys::vfp_ctx,
    vfep: *mut varnish_sys::vfp_entry,
    ptr: *mut c_void,
    len: *mut isize,
) -> varnish_sys::vfp_status {
    let ctx = ctxp.as_mut().unwrap();
    assert_eq!(ctx.magic, varnish_sys::VFP_CTX_MAGIC);
    let vfe = vfep.as_mut().unwrap();
    assert_eq!(vfe.magic, varnish_sys::VFP_ENTRY_MAGIC);

    let buf = std::slice::from_raw_parts_mut(ptr as *mut u8, *len as usize);
    if buf.is_empty() {
            *len = 0;
            return varnish_sys::vfp_status_VFP_OK;
    }

    let reader = (vfe.priv1 as *mut T).as_mut().unwrap();
    match reader.read(buf) {
        Err(e) => {
            let msg = e.to_string();
            // TODO: we should grow a VSL object
            let t = varnish_sys::txt {
                b: msg.as_ptr() as *const c_char,
                e: msg.as_ptr().add(msg.len()) as *const c_char,
            };
            varnish_sys::VSLbt((*(*ctxp).req).vsl, varnish_sys::VSL_tag_e_SLT_Error, t);

            varnish_sys::vfp_status_VFP_ERROR
        },
        Ok(0) => {
            *len = 0;
            varnish_sys::vfp_status_VFP_END
        }
        Ok(l) => {
            *len = l as isize;
            varnish_sys::vfp_status_VFP_OK
        },
    }
}

unsafe extern "C" fn wrap_event<S: Serve<T>, T: Transfer> (
    be: VCLBackendPtr,
    ev: varnish_sys::vcl_event_e,
    ) {
    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).priv_.is_null());
    let backend = (*be).priv_ as *const S;

    (*backend).event(Event::new(ev));
}

unsafe extern "C" fn wrap_list<S: Serve<T>, T: Transfer> (
    ctxp: *const varnish_sys::vrt_ctx,
    be: VCLBackendPtr,
    vsbp: *mut varnish_sys::vsb,
    detailed: i32,
    json: i32,
    ) {
    let mut ctx = Ctx::new(ctxp as *mut varnish_sys::vrt_ctx);
    let mut vsb = Vsb::new(vsbp);
    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).priv_.is_null());
    let backend = (*be).priv_ as *const S;

    (*backend).list(&mut ctx, &mut vsb, detailed != 0, json != 0);
}

unsafe extern "C" fn wrap_panic<S: Serve<T>, T: Transfer> (
    be: VCLBackendPtr,
    vsbp: *mut varnish_sys::vsb,
    ) {
    let mut vsb = Vsb::new(vsbp);

    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).priv_.is_null());
    let backend = (*be).priv_ as *const S;

    (*backend).panic(&mut vsb);
}

unsafe extern "C" fn wrap_pipe<S: Serve<T>, T: Transfer> (
    ctxp: *const varnish_sys::vrt_ctx,
    be: VCLBackendPtr,
    ) -> varnish_sys::stream_close_t {
    let mut ctx = Ctx::new(ctxp as *mut varnish_sys::vrt_ctx);
    assert!(!(*ctxp).req.is_null());
    assert_eq!((*(*ctxp).req).magic, varnish_sys::REQ_MAGIC);
    assert!(!(*(*ctxp).req).sp.is_null());
    assert_eq!((*(*(*ctxp).req).sp).magic, varnish_sys::SESS_MAGIC);
    let fd = (*(*(*ctxp).req).sp).fd;
    assert!(fd != 0);
    let tcp_stream = TcpStream::from_raw_fd(fd);

    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).priv_.is_null());
    let backend = (*be).priv_ as *const S;

    sc_to_ptr((*backend).pipe(&mut ctx, tcp_stream))
}

unsafe extern "C" fn wrap_gethdrs<S: Serve<T>, T: Transfer> (
    ctxp: *const varnish_sys::vrt_ctx,
    be: VCLBackendPtr,
) -> ::std::os::raw::c_int {
    let mut ctx = Ctx::new(ctxp as *mut varnish_sys::vrt_ctx);
    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).vcl_name.is_null());
    assert!(!(*be).priv_.is_null());
    assert!(!(*be).vdir.is_null());
    assert_eq!((*(*be).vdir).magic, varnish_sys::VCLDIR_MAGIC);

    let backend = (*be).priv_ as *const S;
    match (*backend).get_headers(&mut ctx) {
        Ok(res) => {

            // default to HTTP/1.1 200 if the backend didn't provide anything
            let beresp = ctx.http_beresp.as_mut().unwrap();
            if beresp.status().is_none() {
                beresp.set_status(200);
            }
            if beresp.proto().is_none() {
                if let Err(e) = beresp.set_proto("HTTP/1.1") {
                    ctx.fail(&format!("{}: {}", (*backend).get_type(), e));
                    return 1;
                }
            }

            let htc = varnish_sys::WS_Alloc(
                (*ctx.raw.bo).ws.as_mut_ptr(),
                std::mem::size_of::<varnish_sys::http_conn>() as u32,
                ) as *mut varnish_sys::http_conn;
            if htc.is_null() {
                ctx.fail(&format!("{}: insuficient workspace", (*backend).get_type()));
                return -1;
            }
            (*htc).magic = varnish_sys::HTTP_CONN_MAGIC;
            (*htc).doclose = &varnish_sys::SC_REM_CLOSE[0];
            (*htc).content_length = 0;
            match res {
                None => {
                    (*htc).body_status = varnish_sys::BS_NONE.as_ptr();
                },
                Some(transfer) => {
                    match transfer.len() {
                        None => {
                            (*htc).body_status = varnish_sys::BS_CHUNKED.as_ptr();
                        },
                        Some(0) => {
                            (*htc).body_status = varnish_sys::BS_NONE.as_ptr();
                        },
                        Some(l) => {
                            (*htc).body_status = varnish_sys::BS_LENGTH.as_ptr();
                            (*htc).content_length = l as isize;
                        }
                    };
                    (*htc).priv_ = Box::into_raw(Box::new(transfer)) as *mut std::ffi::c_void;
                    // build a vfp to wrap the Transfer object if there's something to push
                    if (*htc).body_status != varnish_sys::BS_NONE.as_ptr() {
                        let vfp = varnish_sys::WS_Alloc(
                            (*ctx.raw.bo).ws.as_mut_ptr(),
                            std::mem::size_of::<varnish_sys::vfp>() as u32,
                            ) as *mut varnish_sys::vfp;
                        if vfp.is_null() {
                            ctx.fail(&format!("{}: insuficient workspace", (*backend).get_type()));
                            return -1;
                        }
                        let t = match WS::new((*ctx.raw.bo).ws.as_mut_ptr()).copy_bytes_with_null(&(*backend).get_type()) {
                            Err(_) => {
                                ctx.fail(&format!("{}: insuficient workspace", (*backend).get_type()));
                                return -1;
                            },
                            Ok(s) => s,
                        };
                        (*vfp).name = t.as_ptr() as *const c_char;
                        (*vfp).init = None;
                        (*vfp).pull = Some(vfp_pull::<T>);
                        (*vfp).fini = None;
                        (*vfp).priv1 = ptr::null();
                        let vfe = varnish_sys::VFP_Push((*ctx.raw.bo).vfc, vfp);
                        if vfe.is_null() {
                            ctx.fail(&format!("{}: couldn't insert vfp", (*backend).get_type()));
                            return -1;
                        }
                        // we don't need to clean (*vfe).priv1 at the vfp level, the backend will
                        // do it in wrap_finish
                        (*vfe).priv1 = (*htc).priv_ ;
                    }

                }
            }

            (*ctx.raw.bo).htc = htc;
            0
        },
        Err(s) => {
            ctx.fail(&s.to_string());
            1
        },
    }
}

unsafe extern "C" fn wrap_healthy<S: Serve<T>, T: Transfer>(
    ctxp: *const varnish_sys::vrt_ctx,
    be: varnish_sys::VCL_BACKEND,
    changed: *mut varnish_sys::VCL_TIME,
) -> varnish_sys::VCL_BOOL {
    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).priv_.is_null());

    let mut ctx = Ctx::new(ctxp as *mut varnish_sys::vrt_ctx);
    let backend = (*be).priv_ as *const Backend<S, T>;
    let (healthy, when) = (*backend).inner.healthy(&mut ctx);
    if !changed.is_null() {
        *changed = when.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64();
    }
    if healthy {
        1
    } else {
        0
    }
}

unsafe extern "C" fn wrap_getip<T: Transfer> (
    ctxp: *const varnish_sys::vrt_ctx,
    _be: varnish_sys::VCL_BACKEND,
) -> varnish_sys::VCL_IP {
    assert!(!ctxp.is_null());
    assert_eq!((*ctxp).magic, varnish_sys::VRT_CTX_MAGIC);
    assert!(!(*ctxp).bo.is_null());
    assert_eq!((*(*ctxp).bo).magic, varnish_sys::BUSYOBJ_MAGIC);
    let bo = *(*ctxp).bo;
    assert!(!bo.htc.is_null());
    assert_eq!((*bo.htc).magic, varnish_sys::BUSYOBJ_MAGIC);
    assert!(!(*bo.htc).priv_.is_null());

    let mut ctx = Ctx::new(ctxp as *mut varnish_sys::vrt_ctx);

    let transfer = (*bo.htc).priv_ as *const T;
    match (*transfer).get_ip().and_then(|ip| ip.into_vcl(&mut ctx.ws).map_err(|e| e.into())) {
        Err(e) => { 
            ctx.fail(&format!("{}", e));
            std::ptr::null()
        }
        Ok(p) => p,
    }
}

unsafe extern "C" fn wrap_finish<S: Serve<T>, T: Transfer> (
    ctxp: *const varnish_sys::vrt_ctx,
    be: VCLBackendPtr,
    ) {
    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).priv_.is_null());

    // drop the Transfer
    let htc = (*(*ctxp).bo).htc;
    if !(*htc).priv_.is_null() {
        drop(Box::from_raw((*htc).priv_ as *mut T));
    }
    (*(*ctxp).bo).htc = ptr::null_mut();

    let backend = (*be).priv_ as *const Backend<S, T>;
    (*backend).inner.finish(&mut Ctx::new(ctxp as *mut varnish_sys::vrt_ctx));
}

impl<S: Serve<T>, T: Transfer> Drop for Backend<S, T> {
    fn drop(&mut self) {
        unsafe { 
            varnish_sys::VRT_DelDirector(&mut self.bep);
        };
    }
}

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

pub fn sc_to_ptr(sc: StreamClose) -> varnish_sys::stream_close_t {
    unsafe {
        match sc {
            StreamClose::RemClose      =>  &varnish_sys::SC_REM_CLOSE as *const _,
            StreamClose::ReqClose      =>  &varnish_sys::SC_REQ_CLOSE as *const _,
            StreamClose::ReqHttp10     =>  &varnish_sys::SC_REQ_HTTP10 as *const _,
            StreamClose::RxBad         =>  &varnish_sys::SC_RX_BAD as *const _,
            StreamClose::RxBody        =>  &varnish_sys::SC_RX_BODY as *const _,
            StreamClose::RxJunk        =>  &varnish_sys::SC_RX_JUNK as *const _,
            StreamClose::RxOverflow    =>  &varnish_sys::SC_RX_OVERFLOW as *const _,
            StreamClose::RxTimeout     =>  &varnish_sys::SC_RX_TIMEOUT as *const _,
            StreamClose::RxCloseIdle   =>  &varnish_sys::SC_RX_CLOSE_IDLE as *const _,
            StreamClose::TxPipe        =>  &varnish_sys::SC_TX_PIPE as *const _,
            StreamClose::TxError       =>  &varnish_sys::SC_TX_ERROR as *const _,
            StreamClose::TxEof         =>  &varnish_sys::SC_TX_EOF as *const _,
            StreamClose::RespClose     =>  &varnish_sys::SC_RESP_CLOSE as *const _,
            StreamClose::Overload      =>  &varnish_sys::SC_OVERLOAD as *const _,
            StreamClose::PipeOverflow  =>  &varnish_sys::SC_PIPE_OVERFLOW as *const _,
            StreamClose::RangeShort    =>  &varnish_sys::SC_RANGE_SHORT as *const _,
            StreamClose::ReqHttp20     =>  &varnish_sys::SC_REQ_HTTP20 as *const _,
            StreamClose::VclFailure    =>  &varnish_sys::SC_VCL_FAILURE as *const _,

        }
    }
}
