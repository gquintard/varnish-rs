#[varnish::vmod]
mod err_fn_args_dups {
    pub fn good(i: i64) {}
    pub fn dup_ctx(ctx: &Ctx, i: i64, ctx2: &Ctx) {}
    pub fn dup_ctx2(ctx: &Ctx, i: i64, ctx2: &mut Ctx) {}
    pub fn dup_shared_vcl(#[shared_per_vcl] a: Option<&i64>, #[shared_per_vcl] b: Option<&i64>) {}
    pub fn dup_shared_task(
        #[shared_per_task] a: Option<Box<i64>>,
        #[shared_per_task] b: Option<Box<i64>>,
    ) {
    }
    #[event]
    pub fn dup_event(a: Event, b: Event) {}
}

fn main() {}
