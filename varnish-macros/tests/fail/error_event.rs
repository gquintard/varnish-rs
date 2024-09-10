#[varnish::vmod]
mod event_dup {
    #[event]
    pub fn event_fn1() {}

    #[event]
    pub fn event_fn2() {}
}

fn main() {}
