#![expect(unused_variables)]

use varnish::vmod;

fn main() {}

#[vmod]
mod event2 {
    use varnish::vcl::{Ctx, Event};

    #[event]
    pub fn on_event(ctx: &Ctx, event: Event) -> Result<(), &'static str> {
        Ok(())
    }
}
