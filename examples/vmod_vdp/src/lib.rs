varnish::boilerplate!();

use std::ffi::CStr;

use varnish::ffi;
use varnish::vcl::{new_vdp, Ctx, Event, InitResult, PushAction, PushResult, VDPCtx, VPriv, VDP};

varnish::vtc!(test01);

// declare a new struct that will buffer the response body
#[derive(Default)]
struct Flipper {
    body: Vec<u8>,
}

// implement the actual behavior of the VDP
impl VDP for Flipper {
    // return our id
    fn name() -> &'static CStr {
        c"flipper"
    }

    // `new` is called when the VCL specifies "flipper" in `resp.filters`
    // just return a default struct, thanks to the derive macro
    fn new(_: &mut Ctx, _: &mut VDPCtx, _oc: *mut ffi::objcore) -> InitResult<Self> {
        InitResult::Ok(Flipper::default())
    }

    // buffer everything, then reverse the buffer, and send it, easy
    fn push(&mut self, ctx: &mut VDPCtx, act: PushAction, buf: &[u8]) -> PushResult {
        // ingest everything we're given
        self.body.extend_from_slice(buf);

        // nod along if it isn't the last call
        if !matches!(act, PushAction::End) {
            return PushResult::Ok;
        }

        // flip the whole body
        self.body.reverse();
        // send
        ctx.push(act, &self.body)
    }
}

pub unsafe fn event(
    ctx: &mut Ctx,
    vp: &mut VPriv<ffi::vdp>,
    event: Event,
) -> Result<(), &'static str> {
    match event {
        // on load, create the VDP C struct, save it into a priv, they register it
        Event::Load => {
            vp.store(new_vdp::<Flipper>());
            ffi::VRT_AddVDP(ctx.raw, vp.as_ref().unwrap());
        }
        // on discard, deregister the VDP, but don't worry about cleaning it, it'll be done by
        // Varnish automatically
        Event::Discard => ffi::VRT_RemoveVDP(ctx.raw, vp.as_ref().unwrap()),
        _ => (),
    }
    Ok(())
}
