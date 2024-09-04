varnish::boilerplate!();

use varnish::ffi;
use varnish::vcl::ctx::{Ctx, Event};
use varnish::vcl::processor::{new_vfp, InitResult, PullResult, VFPCtx, VFP};
use varnish::vcl::vpriv::VPriv;

varnish::vtc!(test01);

// here, we don't actually need a struct to hold data, just to implement some methods
struct Lower {}

// implement the actual behavior of the VFP
impl VFP for Lower {
    // return our id, adding the NULL character to avoid confusing the C layer
    fn name() -> &'static str {
        "lower\0"
    }

    // `new` is called when the VCL specifies "lower" in `beresp.filters`
    fn new(_: &mut Ctx, _: &mut VFPCtx) -> InitResult<Self> {
        InitResult::Ok(Lower {})
    }

    fn pull(&mut self, ctx: &mut VFPCtx, buf: &mut [u8]) -> PullResult {
        let pull_res = ctx.pull(buf);
        let len = match pull_res {
            PullResult::Ok(len) => len,
            PullResult::End(len) => len,
            _ => return pull_res,
        };

        // iterate over the written buffer, and lowercase each element
        for e in buf[..len].iter_mut() {
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
            ffi::VRT_AddVFP(ctx.raw, vp.as_ref().unwrap())
        }
        // on discard, deregister the VFP, but don't worry about cleaning it, it'll be done by
        // Varnish automatically
        Event::Discard => ffi::VRT_RemoveVFP(ctx.raw, vp.as_ref().unwrap()),
        _ => (),
    }
    Ok(())
}
