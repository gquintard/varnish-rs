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
varnish::vtc!(test01);
