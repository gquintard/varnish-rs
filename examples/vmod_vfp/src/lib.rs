use std::ffi::CStr;

use varnish::vcl::{Ctx, FetchProcCtx, FetchProcessor, InitResult, PullResult};

varnish::run_vtc_tests!("tests/*.vtc");

/// Manipulate `resp.body`
///
/// Varnish Fetch Processors allow a vmod writer to insert themselves into a delivery
/// pipeline and alter an object body as it is being received from the backend.
/// In this vmod, we simply lowercase the ascii letters using a filter processor (VFP) named "lower".
#[varnish::vmod(docs = "README.md")]
mod vfp {
    use varnish::vcl::{Event, FetchFilters};

    use super::Lower;

    /// We need the event function here to declare our VFP.
    /// However, there's no "manual" VCL function for us to implement here,
    /// loading the vmod is sufficient to add the VDP to the list of available processors,
    /// and we'll set it on a per-request basis using `beresp.filters` in VCL.
    #[event]
    pub fn event(vfp: &mut FetchFilters, event: Event) {
        if let Event::Load = event {
            vfp.register::<Lower>();
        }
    }
}

// here, we don't actually need a struct to hold data, just to implement some methods
struct Lower {}

// implement the actual behavior of the VFP
impl FetchProcessor for Lower {
    // return our id
    fn name() -> &'static CStr {
        c"lower"
    }

    // `new` is called when the VCL specifies "lower" in `beresp.filters`
    fn new(_: &mut Ctx, _: &mut FetchProcCtx) -> InitResult<Self> {
        InitResult::Ok(Lower {})
    }

    fn pull(&mut self, ctx: &mut FetchProcCtx, buf: &mut [u8]) -> PullResult {
        let pull_res = ctx.pull(buf);
        let (PullResult::End(len) | PullResult::Ok(len)) = pull_res else {
            return pull_res;
        };

        // iterate over the written buffer, and lowercase each element
        for ch in &mut buf[..len] {
            ch.make_ascii_lowercase();
        }

        pull_res
    }
}
