use std::ffi::{CStr, CString};
use std::ffi::c_char;
use std::io::Read;
use std::ptr;

use anyhow::{anyhow, Result};

use crate::vcl::ws::WS;
use crate::vcl::ctx::Ctx;


pub trait Transfer {
    fn len(&self) -> Option<usize> {None}
    fn ip(&self) -> Option<std::net::IpAddr> {None}
}

pub trait Serve<T: Transfer + Read> {
    fn get_type(&self) -> String;
    fn get_headers(&self, ctx: &mut Ctx) -> Result<Option<T>>;
    fn finish(&self, _ctx: &mut Ctx) {}
//        event: None,
//        healthy: None,
//        http1pipe: None,
//        list: None,
//        panic: None,
//        resolve: None,
}

use std::os::raw::c_void;
use varnish_sys::ssize_t;
pub unsafe extern "C" fn vfp_pull<T: Read>(
    ctxp: *mut varnish_sys::vfp_ctx,
    vfep: *mut varnish_sys::vfp_entry,
    ptr: *mut c_void,
    len: *mut ssize_t,
) -> varnish_sys::vfp_status {
    let ctx = ctxp.as_mut().unwrap();
    assert_eq!(ctx.magic, varnish_sys::VFP_CTX_MAGIC);
    let vfe = vfep.as_mut().unwrap();
    assert_eq!(vfe.magic, varnish_sys::VFP_ENTRY_MAGIC);

    let buf = std::slice::from_raw_parts_mut(ptr as *mut u8, *len as usize);
    if buf.len() == 0 {
            *len = 0;
            return varnish_sys::vfp_status_VFP_OK;
    }

    let reader = (vfe.priv1 as *mut T).as_mut().unwrap();
    match reader.read(buf) {
        Err(e) => varnish_sys::vfp_status_VFP_ERROR, // TODO: log error
        Ok(0) => {
            *len = 0;
            varnish_sys::vfp_status_VFP_END
        }
        Ok(l) => {
            *len = l as ssize_t;
            varnish_sys::vfp_status_VFP_OK
        },
    }
}

unsafe extern "C" fn wrap_gethdrs<S: Serve<T>, T: Transfer + Read> (
    ctxp: *const varnish_sys::vrt_ctx,
    be: varnish_sys::VCL_BACKEND,
) -> ::std::os::raw::c_int {
    let mut ctx = Ctx::new(ctxp as *mut varnish_sys::vrt_ctx);
    assert!(!be.is_null());
    assert_eq!((*be).magic, varnish_sys::DIRECTOR_MAGIC);
    assert!(!(*be).vcl_name.is_null());
    assert!(!(*be).priv_.is_null());
    assert!(!(*be).vdir.is_null());
    assert_eq!((*(*be).vdir).magic, varnish_sys::VCLDIR_MAGIC);
    assert!(!(*(*be).vdir).methods.is_null());

    if (*(*(*be).vdir).methods).gethdrs.is_none() {
            ctx.fail(&format!("backend {} has no gethdrs method", CStr::from_ptr((*be).vcl_name).to_str().unwrap() ));
            return 1;
    }
    let backend = (*be).priv_ as *const S;
    match (*backend).get_headers(&mut ctx) {
        Ok(res) => {
            let htc = varnish_sys::WS_Alloc(
                (*ctx.raw.bo).ws.as_mut_ptr(),
                std::mem::size_of::<varnish_sys::http_conn>() as u32,
                ) as *mut varnish_sys::http_conn;
            if htc.is_null() {
                ctx.fail("fileserver: insuficient workspace");
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
                            (*htc).content_length = l as i64;
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
                            ctx.fail(&format!("{}: insuficient workspace", backend.as_ref().unwrap().get_type()));
                            return -1;
                        }
                        let t = match WS::new((*ctx.raw.bo).ws.as_mut_ptr()).copy_bytes_with_null(&backend.as_ref().unwrap().get_type()) {
                            Err(_) => {
                                ctx.fail(&format!("{}: insuficient workspace", backend.as_ref().unwrap().get_type()));
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
                            ctx.fail(&format!("{}: couldn't insert vfp", backend.as_ref().unwrap().get_type()));
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

unsafe extern "C" fn wrap_finish<S: Serve<T>, T: Transfer + Read> (
    ctxp: *const varnish_sys::vrt_ctx,
    be: varnish_sys::VCL_BACKEND
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
    (*backend).info.finish(&mut Ctx::new(ctxp as *mut varnish_sys::vrt_ctx));
}

pub struct Backend<S: Serve<T>, T: Transfer + Read> {
    bep: *const varnish_sys::director,
    methods: Box<varnish_sys::vdi_methods>,
    info: Box<S>,
    type_: CString,
    name: CString,
    phantom: std::marker::PhantomData<T>,
}

impl<S: Serve<T>, T: Transfer + Read> Backend<S, T> {
    pub fn vcl_ptr(&self) -> *const varnish_sys::director {
        self.bep
    }
}

impl<S: Serve<T>, T: Transfer + Read> Drop for Backend<S, T> {
    fn drop(&mut self) {
        unsafe { 
            varnish_sys::VRT_DelDirector(&mut self.bep);
        };
    }
}

pub struct BackendBuilder<S, T> {
    methods: Box<varnish_sys::vdi_methods>,
    info: Box<S>,
    name: CString,
    phantom: std::marker::PhantomData<T>,
}

impl<S: Serve<T>, T: Transfer + Read> BackendBuilder<S, T> {
    pub fn new(n: &str, info: S) -> Result<Self> {
        Ok(BackendBuilder {
            info: Box::new(info),
            name: CString::new(n)?,
            methods: Box::new(varnish_sys::vdi_methods {
                magic: varnish_sys::VDI_METHODS_MAGIC,
                finish: Some(wrap_finish::<S, T>),
                ..Default::default()
            }),
            phantom: std::marker::PhantomData,
        })
    }
    pub fn enable_get_headers(mut self) -> Self {
        self.methods.gethdrs = Some(wrap_gethdrs::<S, T>);
        self
    }

    pub fn build(mut self, ctx: &mut Ctx) -> Result<Backend<S, T>> {
        let type_ = CString::new(&*self.info.get_type())?;
        let bep = unsafe {
            varnish_sys::VRT_AddDirector(
                ctx.raw,
                &*self.methods,
                &mut *self.info as *mut S as *mut std::ffi::c_void,
                self.name.as_ptr() as *const c_char,
            )
        };
        if bep.is_null() {
            return Err(anyhow!("VRT_AddDirector return null while creating {}", self.name.into_string().unwrap()));
        }
        Ok(Backend {
            bep,
            type_,
            methods: self.methods,
            name: self.name,
            info: self.info,
            phantom: std::marker::PhantomData,
        })
    }
//    pub fn enable_get_ip(&mut self) { self.methods.gethdrs = Some(wrap_getip::<T>); }
//    pub fn enable_get_healthy(&mut self) { self.methods.gethdrs = Some(wrap_healthy::<T>); }
//    pub fn enable_get_http1pipe(&mut self) { self.methods.gethdrs = Some(wrap_http1pipe::<T>); }
}
