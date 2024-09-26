varnish::boilerplate!();

use std::ffi::CStr;

use varnish::ffi;
use varnish::vcl::{new_vfp, Ctx, Event, InitResult, PullResult, VFPCtx, VPriv, VFP};

varnish::run_vtc_tests!("tests/*.vtc");

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

pub unsafe fn event(
    ctx: &mut Ctx,
    vp: &mut VPriv<ffi::vfp>,
    event: Event,
) -> Result<(), &'static str> {
    match event {
        // on load, create the VFP C struct, save it into a priv, they register it
        Event::Load => {
            vp.store(new_vfp::<Lower>());
            ffi::VRT_AddVFP(ctx.raw, vp.as_ref().unwrap());
        }
        // on discard, deregister the VFP, but don't worry about cleaning it, it'll be done by
        // Varnish automatically
        Event::Discard => ffi::VRT_RemoveVFP(ctx.raw, vp.as_ref().unwrap()),
        _ => (),
    }
    Ok(())
}
