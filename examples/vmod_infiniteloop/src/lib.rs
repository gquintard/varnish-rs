varnish::run_vtc_tests!("tests/*.vtc");

/// Ignore the restart/retry limits
#[varnish::vmod(docs = "README.md")]
mod infiniteloop {
    use varnish::ffi::{BUSYOBJ_MAGIC, REQ_MAGIC};
    use varnish::vcl::Ctx;

    /// Set the `retries` and `restarts` internal counters to 0.
    /// This is extremely dangerous, and you only use this if you know what you are doing,
    /// and/or love infinite loops.
    ///
    /// # Safety
    /// this function is unsafe from the varnish point of view, doing away with
    /// important safeguards, but it's also unsafe in the rust way: it dereferences
    /// pointers which may lead nowhere
    pub fn reset(ctx: &mut Ctx) {
        unsafe {
            if let Some(req) = ctx.raw.req.as_mut() {
                assert_eq!(req.magic, REQ_MAGIC);
                req.restarts = 0;
            }
            if let Some(bo) = ctx.raw.bo.as_mut() {
                assert_eq!(bo.magic, BUSYOBJ_MAGIC);
                bo.retries = 0;
            }
        }
    }
}
