varnish::boilerplate!();

use std::io::Write;
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
        Ok(req.unset_header(name))
    } else {
        Err("http_req isn't accessible".to_string())
    }
}

pub fn ws_reserve<'a, 'b>(ctx: &'b mut Ctx<'a>, s: &str) -> Result<varnish_sys::VCL_STRING, String> {
    let mut rbuf = ctx.ws.reserve();
    match write!(rbuf.buf, "{} {} {}\0", s, s, s) {
        Ok(()) => {
            let final_buf = rbuf.release(0);
            assert_eq!(final_buf.len(), 3 * s.len() + 3);
            Ok(final_buf.as_ptr() as *const i8)
        },
        _ => Err("workspace issue".to_owned())
    }
}

varnish::vtc!(test01);
varnish::vtc!(test02);
