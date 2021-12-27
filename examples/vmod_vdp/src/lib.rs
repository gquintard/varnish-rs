varnish::boilerplate!();

use varnish::vcl::ctx::Ctx;
use varnish::vcl::ctx::Event;
use varnish::vcl::processor::{new_vdp, InitResult, PushAction, PushResult, VDPCtx, VDP};
use varnish::vcl::vpriv::VPriv;

varnish::vtc!(test01);

// declare a new struct that will buffer the response body
#[derive(Default)]
struct Flipper {
    body: Vec<u8>,
}

// implement the actual behavior of the VDP
impl VDP for Flipper {
    // return our id, adding the NULL character to avoid confusing the C layer
    fn name() -> &'static str {
        "flipper\0"
    }

    // `new` is called when the VCL specifies "flipper" in `resp.filters`
    // just return an default struct, thanks to the derive macro
    fn new(_ctx: &mut VDPCtx, _oc: *mut varnish_sys::objcore) -> InitResult<Self> {
        InitResult::Ok(Default::default())
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
    vp: &mut VPriv<varnish_sys::vdp>,
    event: Event,
) -> Result<(), &'static str> {
    match event {
        // on load, create the VDP C struct, save it into a priv, they register it
        Event::Load => {
            vp.store(new_vdp::<Flipper>());
            varnish_sys::VRT_AddVDP(ctx.raw, vp.as_ref().unwrap())
        }
        // on discard, deregister the VDP, but don't worry about cleaning it, it'll be done by
        // Varnish automatically
        Event::Discard => varnish_sys::VRT_RemoveVDP(ctx.raw, vp.as_ref().unwrap()),
        _ => (),
    }
    Ok(())
}
