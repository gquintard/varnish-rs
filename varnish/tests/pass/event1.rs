#![expect(unused_variables)]

use varnish::vmod;

fn main() {}

#[vmod]
mod event {
    use varnish::vcl::Event;

    /// Event function - the comment is ignored
    #[event]
    pub fn on_event(
        /// Event argument - the comment is ignored
        event: Event,
    ) {
    }
}
