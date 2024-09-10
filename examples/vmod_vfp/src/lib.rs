use std::ffi::CStr;

use varnish::vcl::{Ctx, InitResult, PullResult, VFPCtx, VFP};

varnish::run_vtc_tests!("tests/*.vtc");

/// Manipulate `resp.body`
///
/// Varnish Fetch Processors allow a vmod writer to insert themselves into a delivery
/// pipeline and alter an object body as it is being received from the backend.
/// In this vmod, we simply lowercase the ascii letters using a VFP named "lower".
#[varnish::vmod(docs = "README.md")]
mod vfp {
    use varnish::ffi;
    use varnish::vcl::{new_vfp, Ctx, Event};

    use crate::Lower;

    /// We need the event function here to declare our VFP.
    /// However, there's no "manual" VCL function for us to implement here,
    /// loading the vmod is sufficient to add the VDP to the list of available processors,
    /// and we'll set it on a per-request basis using `beresp.filters` in VCL.
    #[event]
    pub fn event(ctx: &mut Ctx, #[shared_per_vcl] vp: &mut Option<Box<ffi::vfp>>, event: Event) {
        match event {
            // on load, create the VFP C struct, save it into a priv, they register it
            Event::Load => {
                let instance = Box::new(new_vfp::<Lower>());
                unsafe {
                    ffi::VRT_AddVFP(ctx.raw, instance.as_ref());
                }
                *vp = Some(instance);
            }
            // on discard, deregister the VFP, but don't worry about cleaning it, it'll be done by
            // Varnish automatically
            Event::Discard => {
                if let Some(vp) = vp.as_ref() {
                    unsafe { ffi::VRT_RemoveVFP(ctx.raw, vp.as_ref()) }
                }
            }
            _ => {}
        }
    }
}

// here, we don't actually need a struct to hold data, just to implement some methods
struct Lower {}

// implement the actual behavior of the VFP
impl VFP for Lower {
    // return our id
    fn name() -> &'static CStr {
        c"lower"
    }

    // `new` is called when the VCL specifies "lower" in `beresp.filters`
    fn new(_: &mut Ctx, _: &mut VFPCtx) -> InitResult<Self> {
        InitResult::Ok(Lower {})
    }

    fn pull(&mut self, ctx: &mut VFPCtx, buf: &mut [u8]) -> PullResult {
        let pull_res = ctx.pull(buf);
        let (PullResult::End(len) | PullResult::Ok(len)) = pull_res else {
            return pull_res;
        };

        // iterate over the written buffer, and lowercase each element
        for e in &mut buf[..len] {
            e.make_ascii_lowercase();
        }
        pull_res
    }
}
