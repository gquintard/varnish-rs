varnish::boilerplate!();

use std::time::{Duration, Instant};

use varnish::vcl::ctx::Ctx;
use varnish::vcl::vpriv::VPriv;

varnish::vtc!(test01);

// VPriv can wrap any (possibly custom) struct, here we only need an Instand from std::time.
// Storing and getting is up to the vmod writer but this removes the worry of NULL dereferencing
// and of the memory management
pub fn timestamp(_: &Ctx, vp: &mut VPriv<Instant>) -> Duration {
    // we will need this either way
    let now = Instant::now();

    let interval = match vp.as_ref() {
        // if `.get()` returns None, we just store `now` and interval is 0
        None => Duration::new(0, 0),
        // if there was a value, compute the difference with now
        Some(old_now) => now.duration_since(*old_now),
    };
    // store the current time and return `interval`
    vp.store(now);
    interval
}
