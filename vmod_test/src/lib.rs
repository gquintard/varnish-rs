varnish::boilerplate!();

use std::io::Write;
use std::time::Duration;

use varnish::vcl::ctx::Ctx;

pub fn set_hdr(ctx: &mut Ctx, name: &str, value: &str) -> Result<(), String> {
    if let Some(ref mut req) = ctx.http_req {
        req.set_header(name, value)
    } else {
        Err("http_req isn't accessible".to_string())
    }
}

pub fn unset_hdr(ctx: &mut Ctx, name: &str) -> Result<(), String> {
    if let Some(ref mut req) = ctx.http_req {
        req.unset_header(name);
        Ok(())
    } else {
        Err("http_req isn't accessible".to_string())
    }
}

pub fn ws_reserve<'a, 'b>(
    ctx: &'b mut Ctx<'a>,
    s: &str,
) -> Result<varnish_sys::VCL_STRING, String> {
    let mut rbuf = ctx.ws.reserve();
    match write!(rbuf.buf, "{} {} {}\0", s, s, s) {
        Ok(()) => {
            let final_buf = rbuf.release(0);
            assert_eq!(final_buf.len(), 3 * s.len() + 3);
            Ok(final_buf.as_ptr() as *const i8)
        }
        _ => Err("workspace issue".to_owned()),
    }
}

pub fn out_str(_: &mut Ctx) -> &'static str {
    "str"
}

pub fn out_res_str(_: &mut Ctx) -> Result<&'static str, String> {
    Ok("str")
}

pub fn out_string(_: &mut Ctx) -> String {
    "str".to_owned()
}

pub fn out_res_string(_: &mut Ctx) -> Result<String, String> {
    Ok("str".to_owned())
}

pub fn out_bool(_: &mut Ctx) -> bool {
    true
}

pub fn out_res_bool(_: &mut Ctx) -> Result<bool, String> {
    Ok(true)
}

pub fn out_duration(_: &mut Ctx) -> Duration {
    Duration::new(0, 0)
}

pub fn out_res_duration(_: &mut Ctx) -> Result<Duration, String> {
    Ok(Duration::new(0, 0))
}

// this is a pretty terrible idea, the request body is probably big, and your workspace is tiny,
// but hey, it's a test function
pub fn req_body(ctx: &mut Ctx) -> Result<varnish_sys::VCL_STRING, String> {
    let mut body_chunks = ctx.cached_req_body()?;
    // make sure the body will be null-terminated
    body_chunks.push(b"\0");
    // open a ws reservation and blast the body into it
    let mut r = ctx.ws.reserve();
    for chunk in body_chunks {
        r.buf
            .write(chunk)
            .map_err(|_| "workspace issue".to_owned())?;
    }
    Ok(r.release(0).as_ptr() as *const i8)
}

varnish::vtc!(test01);
varnish::vtc!(test02);
varnish::vtc!(test03);
