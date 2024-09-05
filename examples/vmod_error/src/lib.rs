varnish::boilerplate!();

use std::fs::read_to_string;

use varnish::vcl::ctx::Ctx;

varnish::vtc!(test01);

// no error, just return 0 if anything goes wrong
pub fn cannot_fail(_: &Ctx, fp: &str) -> i64 {
    // try to read the path at fp into a string, but return if there was an error
    let content = match read_to_string(fp) {
        Err(_) => return 0,
        Ok(s) => s,
    };

    // try to convert the string into an i64, if parsing fails, force 0
    // no need to return as the last expression is automatically returned
    content.parse::<i64>().unwrap_or(0)
}

// we call ctx.fail() ourselves, but we still need to return an i64 (which will
// be discarded), so we just convert the 0_u8 returned into an i64 (.into() is
// smart enough to infer the type)
pub fn manual_fail(ctx: &mut Ctx, fp: &str) -> i64 {
    // try to read the path at fp into a string, but return if there was an error
    let content = match read_to_string(fp) {
        Err(_) => {
            return ctx
                .fail("manual_fail: couldn't read file into string")
                .into()
        }
        Ok(s) => s,
    };

    // try to convert the string into an i64
    // no need to return as the last expression is automatically returned
    match content.parse::<i64>() {
        Err(_) => ctx.fail("manual_fail: conversion failed").into(),
        Ok(i) => i,
    }
}

// more idiomatic, we return a Result, and the generated boilerplate will be in charge of
// calling `ctx.fail() and return a dummy value
pub fn result_fail(_: &mut Ctx, fp: &str) -> Result<i64, String> {
    read_to_string(fp) // read the file
        .map_err(|e| format!("result_fail: {e}"))? // convert the error (if any!), into a string
        // the ? will automatically return in case
        // of an error
        .parse::<i64>() // convert
        .map_err(|e| format!("result_fail: {e}")) // map the type, and we are good to
                                                  // automatically return
}
