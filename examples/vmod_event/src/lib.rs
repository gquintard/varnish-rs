varnish::boilerplate!();

use varnish::vcl::{Ctx, Event, LogTag, VPriv};

varnish::vtc!(test01);

// number of times event() was run with Event::Load
// event will increment it and store a VCL-local copy via VPriv, that loaded() will be able to
// retrieve
static mut N: i64 = 0;

pub fn loaded(_: &Ctx, vp: &VPriv<i64>) -> i64 {
    // unwrapping is safe here as we know that the event function will have loaded something
    *vp.as_ref().unwrap()
}

// because we are touching a `static mut` variable, this should be highly unsafe, however, Varnish
// guarantees that event functions are called sequentially in the cli thread, so we'll be fine
pub unsafe fn event(ctx: &mut Ctx, vp: &mut VPriv<i64>, event: Event) -> Result<(), &'static str> {
    // log the event, showing that it implements Debug
    ctx.log(LogTag::Debug, format!("event: {event:?}"));

    // we only care about load events, which is why we don't use `match`
    if matches!(event, Event::Load) {
        N += 1;
        if N == 2 {
            //fail the second load, because reasons
            Err("second load always fail")
        } else {
            // store a copy of n
            vp.store(N);
            Ok(())
        }
    } else {
        Ok(())
    }
}
