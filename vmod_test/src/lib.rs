varnish::boilerplate!();

use std::ffi::CStr;
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use varnish::ffi;
use varnish::vcl::ctx::{Ctx, Event};
use varnish::vcl::processor::{new_vfp, InitResult, PullResult, VFPCtx, VFP};
use varnish::vcl::vpriv::VPriv;
use varnish::vcl::{probe, Result};

varnish::vtc!(test01);
varnish::vtc!(test02);
varnish::vtc!(test03);
varnish::vtc!(test04);
varnish::vtc!(test05);
varnish::vtc!(test06);
varnish::vtc!(test07);

pub fn set_hdr(ctx: &mut Ctx, name: &str, value: &str) -> Result<()> {
    if let Some(ref mut req) = ctx.http_req {
        req.set_header(name, value)
    } else {
        Err("http_req isn't accessible".into())
    }
}

pub fn unset_hdr(ctx: &mut Ctx, name: &str) -> Result<()> {
    if let Some(ref mut req) = ctx.http_req {
        req.unset_header(name);
        Ok(())
    } else {
        Err("http_req isn't accessible".into())
    }
}

pub fn ws_reserve<'a, 'b>(ctx: &'b mut Ctx<'a>, s: &str) -> Result<ffi::VCL_STRING> {
    let mut rbuf = ctx.ws.reserve();
    match write!(rbuf.buf, "{} {} {}\0", s, s, s) {
        Ok(()) => {
            let final_buf = rbuf.release(0);
            assert_eq!(final_buf.len(), 3 * s.len() + 3);
            Ok(final_buf.as_ptr() as *const i8)
        }
        _ => Err("workspace issue".into()),
    }
}

pub fn out_str(_: &mut Ctx) -> &'static str {
    "str"
}

pub fn out_res_str(_: &mut Ctx) -> Result<&'static str> {
    Ok("str")
}

pub fn out_string(_: &mut Ctx) -> String {
    "str".to_owned()
}

pub fn out_res_string(_: &mut Ctx) -> Result<String> {
    Ok("str".to_owned())
}

pub fn out_bool(_: &mut Ctx) -> bool {
    true
}

pub fn out_res_bool(_: &mut Ctx) -> Result<bool> {
    Ok(true)
}

pub fn out_duration(_: &mut Ctx) -> Duration {
    Duration::new(0, 0)
}

pub fn out_res_duration(_: &mut Ctx) -> Result<Duration> {
    Ok(Duration::new(0, 0))
}

pub fn build_ip4(_: &mut Ctx) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(12, 34, 56, 78)), 9012)
}

pub fn build_ip6(_: &mut Ctx) -> SocketAddr {
    SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(
            0x1234, 0x5678, 0x9012, 0x3456, 0x7890, 0x1111, 0x2222, 0x3333,
        )),
        4444,
    )
}

pub fn print_ip(_: &mut Ctx, maybe_ip: Option<SocketAddr>) -> String {
    match maybe_ip {
        None => "0.0.0.0".to_string(),
        Some(ip) => ip.to_string(),
    }
}

// this is a pretty terrible idea, the request body is probably big, and your workspace is tiny,
// but hey, it's a test function
pub fn req_body(ctx: &mut Ctx) -> Result<ffi::VCL_STRING> {
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

pub fn default_arg<'a, 'b>(_ctx: &'b mut Ctx, foo: &'a str) -> &'a str {
    foo
}

pub fn cowprobe_prop<'a, 'b>(_ctx: &'b mut Ctx, probe: Option<probe::COWProbe<'a>>) -> String {
    match probe {
        Some(probe) => format!(
            "{}-{}-{}-{}-{}-{}",
            match probe.request {
                probe::COWRequest::URL(url) => format!("url:{}", &url),
                probe::COWRequest::Text(text) => format!("text:{}", &text),
            },
            probe.threshold,
            probe.timeout.as_secs(),
            probe.interval.as_secs(),
            probe.initial,
            probe.window
        ),
        None => "no probe".to_string(),
    }
}

pub fn probe_prop<'b>(_ctx: &'b mut Ctx, probe: Option<probe::Probe>) -> String {
    match probe {
        Some(probe) => format!(
            "{}-{}-{}-{}-{}-{}",
            match probe.request {
                probe::Request::URL(url) => format!("url:{}", &url),
                probe::Request::Text(text) => format!("text:{}", &text),
            },
            probe.threshold,
            probe.timeout.as_secs(),
            probe.interval.as_secs(),
            probe.initial,
            probe.window
        ),
        None => "no probe".to_string(),
    }
}

// Test issue 20 - null pointer drop
struct VFPTest {
    _buffer: Vec<u8>,
}

// Force a pass here to test to make sure that fini does not panic due to a null priv1 member
impl VFP for VFPTest {
    fn new(_: &mut Ctx, _: &mut VFPCtx) -> InitResult<Self> {
        InitResult::Pass
    }

    fn pull(&mut self, _: &mut VFPCtx, _: &mut [u8]) -> PullResult {
        PullResult::Err
    }

    fn name() -> &'static CStr {
        c"vfptest"
    }
}

pub unsafe fn event(
    ctx: &mut Ctx,
    vp: &mut VPriv<ffi::vfp>,
    event: Event,
) -> std::result::Result<(), &'static str> {
    match event {
        // on load, create the VFP C struct, save it into a priv, they register it
        Event::Load => {
            vp.store(new_vfp::<VFPTest>());
            ffi::VRT_AddVFP(ctx.raw, vp.as_ref().unwrap())
        }
        // on discard, deregister the VFP, but don't worry about cleaning it, it'll be done by
        // Varnish automatically
        Event::Discard => ffi::VRT_RemoveVFP(ctx.raw, vp.as_ref().unwrap()),
        _ => (),
    }
    Ok(())
}
