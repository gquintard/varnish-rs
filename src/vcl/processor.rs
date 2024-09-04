//! Varnish has the ability to modify the body of object leaving its cache using delivery
//! processors, named `VDP` in the C API, and implemented here using the [`VDP`] trait.
//! Processors are linked together and will read, modify and push data down the delivery pipeline.
//!
//! *Note:* The rust wrapper here is pretty thin and the vmod writer will most probably need to have to
//! deal with the raw Varnish internals.

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

use crate::varnish_sys;
use crate::varnish_sys::{objcore, vdp_ctx, vfp_ctx, vfp_entry};
use crate::vcl::ctx::Ctx;

/// passed to [`VDP::push`] to describe special conditions occuring in the pipeline.
#[derive(Debug, Copy, Clone)]
pub enum PushAction {
    /// Nothing special
    None = varnish_sys::vdp_action_VDP_NULL as isize,
    /// The accompanying buffer will be invalidated
    Flush = varnish_sys::vdp_action_VDP_FLUSH as isize,
    /// Last call, and last chance to push bytes, implies `Flush`
    End = varnish_sys::vdp_action_VDP_END as isize,
}

/// The return type for [`VDP::push`]
#[derive(Debug, Copy, Clone)]
pub enum PushResult {
    /// Indicates a failure, the pipeline will be stopped with an error
    Err,
    /// Nothing special, processing should continue
    Ok,
    /// Stop early, without error
    End,
}

/// The return type for [`VFP::pull`]
#[derive(Debug, Copy, Clone)]
pub enum PullResult {
    /// Indicates a failure, the pipeline will be stopped with an error
    Err,
    /// Specify how many bytes were written to the buffer, and that the processor is ready for the
    /// next call
    Ok(usize),
    /// The processor is done, and returns how many bytes were treated
    End(usize),
}

/// The return type for [`VDP::new`] and [`VFP::new`]
#[derive(Debug)]
pub enum InitResult<T> {
    Err(String),
    Ok(T),
    Pass,
}

/// Describes a VDP
pub trait VDP
where
    Self: Sized,
{
    /// Create a new processor, possibly using knowledge from the pipeline, or from the current
    /// request.
    fn new(vrt_ctx: &mut Ctx, vdp_ctx: &mut VDPCtx, oc: *mut objcore) -> InitResult<Self>;
    /// Handle the data buffer from the previous processor. This function generally uses
    /// [`VDPCtx::push`] to push data to the next processor.
    fn push(&mut self, ctx: &mut VDPCtx, act: PushAction, buf: &[u8]) -> PushResult;
    /// The name of the processor.
    ///
    /// **Note:** it must be NULL-terminated as it will be used directly as a C string.
    fn name() -> &'static str;
}

pub unsafe extern "C" fn gen_vdp_init<T: VDP>(
    vrt_ctx: *const varnish_sys::vrt_ctx,
    ctx_raw: *mut vdp_ctx,
    priv_: *mut *mut c_void,
    oc: *mut objcore,
) -> c_int {
    assert_ne!(priv_, ptr::null_mut());
    assert_eq!(*priv_, ptr::null_mut());
    match T::new(
        &mut Ctx::new(vrt_ctx as *mut varnish_sys::vrt_ctx),
        &mut VDPCtx::new(ctx_raw),
        oc,
    ) {
        InitResult::Ok(proc) => {
            *priv_ = Box::into_raw(Box::new(proc)).cast::<c_void>();
            0
        }
        InitResult::Err(_) => -1, // TODO: log error
        InitResult::Pass => 1,
    }
}

pub unsafe extern "C" fn gen_vdp_fini<T: VDP>(
    _: *mut vdp_ctx,
    priv_: *mut *mut c_void,
) -> std::os::raw::c_int {
    if priv_.is_null() {
        return 0;
    }
    assert_ne!(*priv_, ptr::null_mut());
    drop(Box::from_raw((*priv_).cast::<T>()));
    *priv_ = ptr::null_mut();
    0
}

pub unsafe extern "C" fn gen_vdp_push<T: VDP>(
    ctx_raw: *mut vdp_ctx,
    act: varnish_sys::vdp_action,
    priv_: *mut *mut c_void,
    ptr: *const c_void,
    len: isize,
) -> c_int {
    assert_ne!(priv_, ptr::null_mut());
    assert_ne!(*priv_, ptr::null_mut());
    let out_action = match act {
        varnish_sys::vdp_action_VDP_NULL => PushAction::None,
        varnish_sys::vdp_action_VDP_FLUSH => PushAction::Flush,
        varnish_sys::vdp_action_VDP_END => PushAction::End,
        _ => return 1, /* TODO: log */
    };

    let empty_buffer: [u8; 0] = [0; 0];
    let buf = if ptr.is_null() {
        &empty_buffer
    } else {
        std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize)
    };

    match (*(*priv_).cast::<T>()).push(&mut VDPCtx::new(ctx_raw), out_action, buf) {
        PushResult::Err => -1, // TODO: log error
        PushResult::Ok => 0,
        PushResult::End => 1,
    }
}

/// Create a `varnish_sys::vdp` that can be fed to `varnish_sys::VRT_AddVDP`
pub fn new_vdp<T: VDP>() -> varnish_sys::vdp {
    varnish_sys::vdp {
        name: T::name().as_ptr().cast::<c_char>(),
        init: Some(gen_vdp_init::<T>),
        bytes: Some(gen_vdp_push::<T>),
        fini: Some(gen_vdp_fini::<T>),
        priv1: ptr::null(),
    }
}

/// A thin wrapper around a `*mut varnish_sys::vdp_ctx`
pub struct VDPCtx<'a> {
    pub raw: &'a mut vdp_ctx,
}

impl<'a> VDPCtx<'a> {
    /// Check the pointer validity and returns the rust equivalent.
    ///
    /// # Safety
    ///
    /// The caller is in charge of making sure the structure doesn't outlive the pointer.
    pub unsafe fn new(raw: *mut vdp_ctx) -> Self {
        let raw = raw.as_mut().unwrap();
        assert_eq!(raw.magic, varnish_sys::VDP_CTX_MAGIC);
        VDPCtx { raw }
    }

    /// Send buffer down the pipeline
    pub fn push(&mut self, act: PushAction, buf: &[u8]) -> PushResult {
        match unsafe {
            varnish_sys::VDP_bytes(
                self.raw,
                act as std::os::raw::c_uint,
                buf.as_ptr().cast::<c_void>(),
                buf.len() as isize,
            )
        } {
            r if r < 0 => PushResult::Err,
            0 => PushResult::Ok,
            _ => PushResult::End,
        }
    }
}

/// Describes a VFP
pub trait VFP
where
    Self: Sized,
{
    /// Create a new processor, possibly using knowledge from the pipeline
    fn new(_vrt_ctx: &mut Ctx, _vfp_ctx: &mut VFPCtx) -> InitResult<Self> {
        unimplemented!()
    }
    /// Write data into `buf`, generally using `VFP_Suck` to collect data from the previous
    /// processor.
    fn pull(&mut self, ctx: &mut VFPCtx, buf: &mut [u8]) -> PullResult;
    /// The name of the processor.
    ///
    /// **Note:** it must be NULL-terminated as it will be used directly as a C string.
    fn name() -> &'static str {
        unimplemented!()
    }
}

unsafe extern "C" fn wrap_vfp_init<T: VFP>(
    vrt_ctx: *const varnish_sys::vrt_ctx,
    ctxp: *mut vfp_ctx,
    vfep: *mut vfp_entry,
) -> varnish_sys::vfp_status {
    let ctx = ctxp.as_mut().unwrap();
    assert_eq!(ctx.magic, varnish_sys::VFP_CTX_MAGIC);

    let vfe = vfep.as_mut().unwrap();
    assert_eq!(vfe.magic, varnish_sys::VFP_ENTRY_MAGIC);

    match T::new(
        &mut Ctx::new(vrt_ctx as *mut varnish_sys::vrt_ctx),
        &mut VFPCtx::new(ctx),
    ) {
        InitResult::Ok(proc) => {
            vfe.priv1 = Box::into_raw(Box::new(proc)).cast::<c_void>();
            0
        }
        InitResult::Err(_) => -1, // TODO: log the error,
        InitResult::Pass => 1,
    }
}

pub unsafe extern "C" fn wrap_vfp_pull<T: VFP>(
    ctxp: *mut vfp_ctx,
    vfep: *mut vfp_entry,
    ptr: *mut c_void,
    len: *mut isize,
) -> varnish_sys::vfp_status {
    let ctx = ctxp.as_mut().unwrap();
    assert_eq!(ctx.magic, varnish_sys::VFP_CTX_MAGIC);
    let vfe = vfep.as_mut().unwrap();
    assert_eq!(vfe.magic, varnish_sys::VFP_ENTRY_MAGIC);

    let mut empty_buffer: [u8; 0] = [0; 0];
    let buf = if ptr.is_null() {
        empty_buffer.as_mut()
    } else {
        std::slice::from_raw_parts_mut(ptr.cast::<u8>(), *len as usize)
    };
    let obj = vfe.priv1.cast::<T>().as_mut().unwrap();
    match obj.pull(&mut VFPCtx::new(ctx), buf) {
        PullResult::Err => varnish_sys::vfp_status_VFP_ERROR, // TODO: log error
        PullResult::Ok(l) => {
            *len = l as isize;
            varnish_sys::vfp_status_VFP_OK
        }
        PullResult::End(l) => {
            *len = l as isize;
            varnish_sys::vfp_status_VFP_END
        }
    }
}

pub unsafe extern "C" fn wrap_vfp_fini<T: VFP>(ctxp: *mut vfp_ctx, vfep: *mut vfp_entry) {
    let ctx = ctxp.as_mut().unwrap();
    assert_eq!(ctx.magic, varnish_sys::VFP_CTX_MAGIC);
    let vfe = vfep.as_mut().unwrap();
    assert_eq!(vfe.magic, varnish_sys::VFP_ENTRY_MAGIC);

    if vfe.priv1.is_null() {
        return;
    }

    drop(Box::from_raw(vfe.priv1.cast::<T>()));
    vfe.priv1 = ptr::null_mut();
}

/// Create a `varnish_sys::vfp` that can be fed to `varnish_sys::VRT_AddVFP`
pub fn new_vfp<T: VFP>() -> varnish_sys::vfp {
    varnish_sys::vfp {
        name: T::name().as_ptr().cast::<c_char>(),
        init: Some(wrap_vfp_init::<T>),
        pull: Some(wrap_vfp_pull::<T>),
        fini: Some(wrap_vfp_fini::<T>),
        priv1: ptr::null(),
    }
}

/// A thin wrapper around a `*mut varnish_sys::vfp_ctx`
pub struct VFPCtx<'a> {
    pub raw: &'a mut vfp_ctx,
}

impl<'a> VFPCtx<'a> {
    /// Check the pointer validity and returns the rust equivalent.
    ///
    /// # Safety
    ///
    /// The caller is in charge of making sure the structure doesn't outlive the pointer.
    pub unsafe fn new(raw: *mut vfp_ctx) -> Self {
        let raw = raw.as_mut().unwrap();
        assert_eq!(raw.magic, varnish_sys::VFP_CTX_MAGIC);
        VFPCtx { raw }
    }

    /// Pull data from the pipeline
    pub fn pull(&mut self, buf: &mut [u8]) -> PullResult {
        let mut len = buf.len() as isize;
        let max_len = len;

        match unsafe { varnish_sys::VFP_Suck(self.raw, buf.as_ptr() as *mut c_void, &mut len) } {
            varnish_sys::vfp_status_VFP_OK => {
                assert!(len <= max_len);
                assert!(len >= 0);
                PullResult::Ok(len as usize)
            }
            varnish_sys::vfp_status_VFP_END => {
                assert!(len <= max_len);
                assert!(len >= 0);
                PullResult::End(len as usize)
            }
            varnish_sys::vfp_status_VFP_ERROR => PullResult::Err,
            n => panic!("unknown vfp_status: {n}"),
        }
    }
}
