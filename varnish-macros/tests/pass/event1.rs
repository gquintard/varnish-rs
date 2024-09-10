#![allow(unused_variables)]

use varnish::vmod;

fn main() {}

#[vmod]
mod event {
    use varnish::vcl::Event;

    #[event]
    pub fn on_event(event: Event) {
        panic!()
    }
}
