use crate::ffi;
use crate::ffi::{vfp_entry, vrt_ctx};
use crate::vcl::backend::VCLBackendPtr;

pub(crate) unsafe fn validate_vfp_ctx(ctxp: *mut ffi::vfp_ctx) -> &'static mut ffi::vfp_ctx {
    let ctx = ctxp.as_mut().unwrap();
    assert_eq!(ctx.magic, ffi::VFP_CTX_MAGIC);
    ctx
}

pub(crate) unsafe fn validate_vrt_ctx(ctxp: *const vrt_ctx) -> &'static vrt_ctx {
    let ctxp = ctxp.as_ref().unwrap();
    assert_eq!(ctxp.magic, ffi::VRT_CTX_MAGIC);
    ctxp
}

pub(crate) unsafe fn validate_vfp_entry(vfep: *mut vfp_entry) -> &'static mut vfp_entry {
    let vfe = vfep.as_mut().unwrap();
    assert_eq!(vfe.magic, ffi::VFP_ENTRY_MAGIC);
    vfe
}

pub(crate) unsafe fn validate_director(be: VCLBackendPtr) -> &'static ffi::director {
    let be = be.as_ref().unwrap();
    assert_eq!(be.magic, ffi::DIRECTOR_MAGIC);
    be
}

pub(crate) unsafe fn validate_ws(wsp: *mut ffi::ws) -> &'static mut ffi::ws {
    let ws = wsp.as_mut().unwrap();
    assert_eq!(ws.magic, ffi::WS_MAGIC);
    ws
}
