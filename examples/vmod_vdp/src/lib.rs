use std::ffi::CStr;

use varnish::ffi::VdpAction;
use varnish::vcl::{Ctx, DeliveryProcCtx, DeliveryProcessor, InitResult, PushResult};

varnish::run_vtc_tests!("tests/*.vtc");

/// Manipulate `resp.body`
///
/// Varnish Delivery Processors allow a vmod writer to insert themselves into a delivery
/// pipeline and alter an object body as it is being delivered to a client.  In this vmod,
/// the transformation is very simple: we simply send the body backwards using a delivery
/// processor (VDP) named "flipper".
#[varnish::vmod(docs = "README.md")]
mod vdp {
    use varnish::vcl::{DeliveryFilters, Event};

    use super::Flipper;

    /// We need the event function here to declare our VDP.
    /// However, there's no "manual" VCL function for us to implement here,
    /// loading the vmod is sufficient to add the VDP to the list of available processors,
    /// and we'll set it on a per-request basis using `resp.filters` in VCL.
    #[event]
    pub fn event(vdp: &mut DeliveryFilters, event: Event) {
        if let Event::Load = event {
            vdp.register::<Flipper>();
        }
    }
}

// declare a new struct that will buffer the response body
#[derive(Default)]
struct Flipper {
    body: Vec<u8>,
}

// implement the actual behavior of the VDP
impl DeliveryProcessor for Flipper {
    // return our id
    fn name() -> &'static CStr {
        c"flipper"
    }

    // `new` is called when the VCL specifies "flipper" in `resp.filters`
    // just return a default struct, thanks to the derive macro
    fn new(_: &mut Ctx, _: &mut DeliveryProcCtx) -> InitResult<Self> {
        InitResult::Ok(Flipper::default())
    }

    // buffer everything, then reverse the buffer, and send it, easy
    fn push(&mut self, ctx: &mut DeliveryProcCtx, act: VdpAction, buf: &[u8]) -> PushResult {
        // ingest everything we're given
        self.body.extend_from_slice(buf);

        if matches!(act, VdpAction::End) {
            // flip the whole body
            self.body.reverse();
            // send
            ctx.push(act, &self.body)
        } else {
            // nod along if it isn't the last call
            PushResult::Ok
        }
    }
}
