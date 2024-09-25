varnish::boilerplate!();

use std::fs::read_to_string;

use varnish::vcl::Ctx;

varnish::vtc!(test01);

// no error, just return 0 if anything goes wrong
pub fn cannot_fail(_: &Ctx, fp: &str) -> i64 {
    // try to read the path at fp into a string, but return if there was an error
    let Ok(content) = read_to_string(fp) else {
        return 0;
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
    let Ok(content) = read_to_string(fp) else {
        ctx.fail("manual_fail: couldn't read file into string");
        return 0;
    };

    // try to convert the string into an i64
    // no need to return as the last expression is automatically returned
    let Ok(result) = content.parse::<i64>() else {
        ctx.fail("manual_fail: conversion failed");
        return 0;
    };

    result
}

// more idiomatic, we return a Result, and the generated boilerplate will be in charge of
// calling `ctx.fail() and return a dummy value
pub fn result_fail(_: &mut Ctx, fp: &str) -> Result<i64, String> {
    // read the file
    read_to_string(fp)
        // convert the error (if any!), into a string and return right away with the `?` operator
        .map_err(|e| format!("result_fail: {e}"))?
        // try to parse content into i64
        .parse::<i64>()
        // map the error to a string message and return either the parsed integer or that error
        .map_err(|e| format!("result_fail: {e}"))
}
