// even though we won't use it here, we still need to know what the context type is
use varnish::vcl::ctx::Ctx;

// this import is only needed for tests
#[cfg(test)]
use varnish::vcl::ctx::TestCtx;

// we now implement both functionis from vmod.vcc, but with rust types.
// Don't forget to make the function public with "pub" in front of them

// we could skip the return, or even use n.is_even(), but let's pace ourselves
pub fn is_even(_: &Ctx, n: i64) -> bool {
    return n % 2 == 0;
}

// in vmod.vcc, n was an optional INT, so here it translates into a Option<i64>
pub fn captain_obvious(_: &Ctx, opt: Option<i64>) -> String {
    // we need to first "match" to know if a number was provided, if not,
    // return a default message, otherwise, build a custom one
    match opt {
        // no need to return, we are the last expression of the function!
        None => String::from("I was called without an argument"),
        // pattern matching FTW!
        Some(n) => format!("I was given {} as argument", n),
    }
}

// Write some more unit tests
#[test]
fn obviousness() {
    let mut test_ctx = TestCtx::new(100);
    let ctx = test_ctx.ctx();

    assert_eq!(
        "I was called without an argument",
        captain_obvious(&ctx, None)
    );
    assert_eq!(
        "I was given 975322 as argument",
        captain_obvious(&ctx, Some(975322))
    );
}

// Write some more unit tests
#[test]
fn even_test() {
    // we don't use it, but we still need one
    let mut test_ctx = TestCtx::new(100);
    let ctx = test_ctx.ctx();

    assert_eq!(true, is_even(&ctx, 0));
    assert_eq!(true, is_even(&ctx, 1024));
    assert_eq!(false, is_even(&ctx, 421321));
}

// we also want to run test/test01.vtc
varnish::vtc!(test01);
