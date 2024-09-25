varnish::boilerplate!();

use varnish::ffi::{BUSYOBJ_MAGIC, REQ_MAGIC};
use varnish::vcl::Ctx;

varnish::vtc!(test01);

/// # Safety
/// this function is unsafe from the varnish point of view, doing away with
/// important safeguards, but it's also unsafe in the rust way: it dereferences
/// pointers which may lead nowhere
pub unsafe fn reset(ctx: &mut Ctx) {
    if let Some(req) = ctx.raw.req.as_mut() {
        assert_eq!(req.magic, REQ_MAGIC);
        req.restarts = 0;
    }
    if let Some(bo) = ctx.raw.bo.as_mut() {
        assert_eq!(bo.magic, BUSYOBJ_MAGIC);
        bo.retries = 0;
    }
}
