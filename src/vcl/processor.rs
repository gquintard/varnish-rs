//! Varnish has the ability to modify the body of object leaving its cache using delivery
//! processors, named `VDP` in the C API, and implemented here using the [`OutProc`] trait.
//! Processors are linked together and will read, modify and push data down the delivery pipeline.
//!
//! The rust wrapper here is pretty thin and the vmod writer will most probably need to have to
//! deal with the raw Varnish internals.

use std::os::raw::c_void;
use std::ptr;
use std::os::raw::c_int;

use varnish_sys::{objcore, vdp_ctx};

/// passed to [`OutProc::bytes`] to describe special conditions occuring in the pipeline.
#[derive(Debug, Copy, Clone)]
pub enum OutAction {
    /// Nothing special
    None = varnish_sys::vdp_action_VDP_NULL as isize,
    /// The accompanying buffer will be invalidated
    Flush = varnish_sys::vdp_action_VDP_FLUSH as isize,
    /// Last call, and last chance to push bytes, implies `Flush`
    End = varnish_sys::vdp_action_VDP_END as isize,
}

/// The retrun type for [`OutProc::bytes`]
#[derive(Debug, Copy, Clone)]
pub enum OutResult {
    /// Indicates a failure, the pipeline will be stopped with an error
    Error = -1,
    /// Nothing special, processing should continue
    Continue = 0,
    /// Stop early, without error
    Stop = 1,
}

/// Describes a VDP
pub trait OutProc
where
    Self: Sized,
{
    /// Create a new processor, possibly using knowledge from the pipeline, or from the current
    /// request.
    fn new(ctx: &mut OutCtx, oc: *mut varnish_sys::objcore) -> Result<Self, String>;
    /// Handle the data buffer from the previous processor. This function generally uses
    /// [`OutCtx::push_bytes`] to push data to the next processor.
    fn bytes(
        &mut self,
        ctx: &mut OutCtx,
        act: OutAction,
        buf: &[u8],
    ) -> OutResult;
    /// The name of the processor.
    ///
    /// **Note:** it must be NULL-terminated as it will be used directly as a C string.
    fn name() -> &'static str;
}

unsafe extern "C" fn gen_vdp_init<T: OutProc>(
    ctx_raw: *mut vdp_ctx,
    priv_: *mut *mut c_void,
    oc: *mut objcore,
) -> c_int {
    assert_ne!(priv_, ptr::null_mut());
    assert_eq!(*priv_, ptr::null_mut());
    match T::new(&mut OutCtx::new(ctx_raw), oc) {
        Ok(proc) => {
            *priv_ = Box::into_raw(Box::new(proc)) as *mut c_void;
            0
        }
        Err(_) => {
            1 /* TODO: log*/
        }
    }
}

unsafe extern "C" fn gen_vdp_fini<T: OutProc>(
    _: *mut vdp_ctx,
    priv_: *mut *mut c_void,
) -> std::os::raw::c_int {
    assert_ne!(priv_, ptr::null_mut());
    assert_ne!(*priv_, ptr::null_mut());
    Box::from_raw(*priv_ as *mut T);
    *priv_ = ptr::null_mut();
    0
}

unsafe extern "C" fn gen_vdp_bytes<T: OutProc>(
    ctx_raw: *mut vdp_ctx,
    act: varnish_sys::vdp_action,
    priv_: *mut *mut c_void,
    ptr: *const c_void,
    len: varnish_sys::ssize_t,
) -> c_int {
    assert_ne!(priv_, ptr::null_mut());
    assert_ne!(*priv_, ptr::null_mut());
    let obj = (*priv_ as *mut T).as_mut().unwrap();
    let out_action = match act {
        varnish_sys::vdp_action_VDP_NULL => OutAction::None,
        varnish_sys::vdp_action_VDP_FLUSH => OutAction::Flush,
        varnish_sys::vdp_action_VDP_END => OutAction::End,
        _ => return 1, /* TODO: log */
    };
    let buf = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    obj.bytes(&mut OutCtx::new(ctx_raw), out_action, buf) as c_int
}

pub fn new_vdp<T: OutProc>() -> varnish_sys::vdp {
    varnish_sys::vdp {
        name: T::name().as_ptr() as *const i8,
        init: Some(gen_vdp_init::<T>),
        bytes: Some(gen_vdp_bytes::<T>),
        fini: Some(gen_vdp_fini::<T>),
    }
}

/// A thin wrapper around a `*mut varnish_sys::vdp_ctx`
pub struct OutCtx<'a> {
    pub raw: &'a mut varnish_sys::vdp_ctx,
}

impl<'a> OutCtx<'a> {
    /// Check the pointer validity and returns the rust equivalent.
    ///
    /// # Safety
    ///
    /// The caller is in charge of making sure the structure doesn't outlive the pointer.
    pub unsafe fn new(raw: *mut varnish_sys::vdp_ctx) -> Self {
        let raw = raw.as_mut().unwrap();
        assert_eq!(raw.magic, varnish_sys::VDP_CTX_MAGIC);
        OutCtx { raw  }
    }

    /// Send buffer down the pipeline
    pub fn push_bytes(&mut self, act: OutAction, buf: &[u8]) -> OutResult {
        match unsafe { varnish_sys::VDP_bytes(
                self.raw,
                act as std::os::raw::c_uint,
                buf.as_ptr() as *const c_void,
                buf.len() as varnish_sys::ssize_t,
                ) } {
            r if r < 0 => OutResult::Error,
            0 => OutResult::Continue,
            _ => OutResult::Stop,
        }
    }
}
