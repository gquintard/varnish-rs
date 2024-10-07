#[varnish::vmod]
mod empty {}

#[varnish::vmod]
mod err_fn {
    fn non_public() {}
    pub async fn async_fn() {}
    pub unsafe fn unsafe_fn() {}
}

fn main() {}
