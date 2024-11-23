use crate::ffi;
use crate::ffi::{
    director, req, sess, vrt_ctx, ws, DIRECTOR_MAGIC, REQ_MAGIC, SESS_MAGIC, VCL_BACKEND,
    VRT_CTX_MAGIC, WS_MAGIC,
};
#[cfg(not(lts_60))]
use crate::ffi::{vcldir, vfp_ctx, vfp_entry, VCLDIR_MAGIC, VFP_CTX_MAGIC, VFP_ENTRY_MAGIC};

#[cfg(not(lts_60))]
use crate::vcl::{DeliveryFilters, FetchFilters};

#[cfg(not(lts_60))]
pub unsafe fn validate_vfp_ctx(ctxp: *mut vfp_ctx) -> &'static mut vfp_ctx {
    let val = ctxp.as_mut().unwrap();
    assert_eq!(val.magic, VFP_CTX_MAGIC);
    val
}

pub unsafe fn validate_vrt_ctx(ctxp: *const vrt_ctx) -> &'static vrt_ctx {
    let val = ctxp.as_ref().unwrap();
    assert_eq!(val.magic, VRT_CTX_MAGIC);
    val
}

#[cfg(not(lts_60))]
pub unsafe fn validate_vfp_entry(vfep: *mut vfp_entry) -> &'static mut vfp_entry {
    let val = vfep.as_mut().unwrap();
    assert_eq!(val.magic, VFP_ENTRY_MAGIC);
    val
}

pub unsafe fn validate_director(be: VCL_BACKEND) -> &'static director {
    let val = be.0.as_ref().unwrap();
    assert_eq!(val.magic, DIRECTOR_MAGIC);
    val
}

pub unsafe fn validate_ws(wsp: *mut ws) -> &'static mut ws {
    let val = wsp.as_mut().unwrap();
    assert_eq!(val.magic, WS_MAGIC);
    val
}

#[cfg(not(lts_60))]
pub unsafe fn validate_vdir(be: &director) -> &'static mut vcldir {
    let val = be.vdir.as_mut().unwrap();
    assert_eq!(val.magic, VCLDIR_MAGIC);
    val
}

impl vrt_ctx {
    pub fn validated_req(&mut self) -> &mut req {
        let val = unsafe { self.req.as_mut().unwrap() };
        assert_eq!(val.magic, REQ_MAGIC);
        val
    }

    #[cfg(not(lts_60))]
    pub fn fetch_filters<'c, 'f>(
        &'c self,
        filters: &'f mut Vec<Box<ffi::vfp>>,
    ) -> FetchFilters<'c, 'f> {
        FetchFilters::<'c, 'f>::new(self, filters)
    }

    #[cfg(not(lts_60))]
    pub fn delivery_filters<'c, 'f>(
        &'c self,
        filters: &'f mut Vec<Box<ffi::vdp>>,
    ) -> DeliveryFilters<'c, 'f> {
        DeliveryFilters::<'c, 'f>::new(self, filters)
    }
}

impl req {
    pub fn validated_session(&mut self) -> &sess {
        let val = unsafe { self.sp.as_ref().unwrap() };
        assert_eq!(val.magic, SESS_MAGIC);
        val
    }
}
