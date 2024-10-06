#![allow(clippy::unnecessary_wraps)]

use std::ffi::CStr;

use varnish::vcl::{Ctx, InitResult, PullResult, VFPCtx, VFP};
use varnish::vmod;

varnish::run_vtc_tests!("tests/*.vtc");

/// Test vmod
#[vmod(docs = "README.md")]
mod rustest {
    use std::io::Write;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::time::Duration;

    use varnish::ffi;
    use varnish::ffi::VCL_STRING;
    use varnish::vcl::{new_vfp, COWProbe, COWRequest, Ctx, Event, Probe, Request, VclError};

    use super::VFPTest;

    pub fn set_hdr(ctx: &mut Ctx, name: &str, value: &str) -> Result<(), VclError> {
        if let Some(ref mut req) = ctx.http_req {
            Ok(req.set_header(name, value)?)
        } else {
            Err("http_req isn't accessible".into())
        }
    }

    pub fn unset_hdr(ctx: &mut Ctx, name: &str) -> Result<(), &'static str> {
        if let Some(ref mut req) = ctx.http_req {
            req.unset_header(name);
            Ok(())
        } else {
            Err("http_req isn't accessible")
        }
    }

    pub fn ws_reserve(ctx: &mut Ctx, s: &str) -> Result<VCL_STRING, &'static str> {
        let mut rbuf = ctx.ws.reserve();
        match write!(rbuf.buf, "{s} {s} {s}\0") {
            Ok(()) => {
                let final_buf = rbuf.release(0);
                assert_eq!(final_buf.len(), 3 * s.len() + 3);
                Ok(VCL_STRING(final_buf.as_ptr() as *const i8))
            }
            _ => Err("workspace issue"),
        }
    }

    pub fn out_str() -> &'static str {
        "str"
    }

    pub fn out_res_str() -> Result<&'static str, &'static str> {
        Ok("str")
    }

    pub fn out_string() -> String {
        "str".to_owned()
    }

    pub fn out_res_string() -> Result<String, &'static str> {
        Ok("str".to_owned())
    }

    pub fn out_bool() -> bool {
        true
    }

    pub fn out_res_bool() -> Result<bool, &'static str> {
        Ok(true)
    }

    pub fn out_duration() -> Duration {
        Duration::new(0, 0)
    }

    pub fn out_res_duration() -> Result<Duration, &'static str> {
        Ok(Duration::new(0, 0))
    }

    pub fn build_ip4() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(12, 34, 56, 78)), 9012)
    }

    pub fn build_ip6() -> SocketAddr {
        SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(
                0x1234, 0x5678, 0x9012, 0x3456, 0x7890, 0x1111, 0x2222, 0x3333,
            )),
            4444,
        )
    }

    pub fn print_ip(maybe_ip: Option<SocketAddr>) -> String {
        match maybe_ip {
            None => "0.0.0.0".to_string(),
            Some(ip) => ip.to_string(),
        }
    }

    // this is a pretty terrible idea, the request body is probably big, and your workspace is tiny,
    // but hey, it's a test function
    pub fn req_body(ctx: &mut Ctx) -> Result<VCL_STRING, VclError> {
        let mut body_chunks = ctx.cached_req_body()?;
        // make sure the body will be null-terminated
        body_chunks.push(b"\0");
        // open a ws reservation and blast the body into it
        let mut r = ctx.ws.reserve();
        for chunk in body_chunks {
            r.buf.write_all(chunk).map_err(|_| "workspace issue")?;
        }
        Ok(VCL_STRING(r.release(0).as_ptr() as *const i8))
    }

    pub fn default_arg(#[arg(default = "foo")] arg: &str) -> &str {
        arg
    }

    pub fn cowprobe_prop(probe: Option<COWProbe<'_>>) -> String {
        match probe {
            Some(probe) => format!(
                "{}-{}-{}-{}-{}-{}",
                match probe.request {
                    COWRequest::URL(url) => format!("url:{url}"),
                    COWRequest::Text(text) => format!("text:{text}"),
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

    pub fn probe_prop(probe: Option<Probe>) -> String {
        match probe {
            Some(probe) => format!(
                "{}-{}-{}-{}-{}-{}",
                match probe.request {
                    Request::URL(url) => format!("url:{url}"),
                    Request::Text(text) => format!("text:{text}"),
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

    #[event]
    pub fn event(
        ctx: &mut Ctx,
        #[shared_per_vcl] vp: &mut Option<Box<ffi::vfp>>,
        event: Event,
    ) -> Result<(), &'static str> {
        match event {
            // on load, create the VFP C struct, save it into a priv, they register it
            Event::Load => {
                let instance = Box::new(new_vfp::<VFPTest>());
                unsafe {
                    ffi::VRT_AddVFP(ctx.raw, instance.as_ref());
                }
                *vp = Some(instance);
            }
            // on discard, deregister the VFP, but don't worry about cleaning it, it'll be done by
            // Varnish automatically
            Event::Discard => {
                if let Some(vp) = vp.as_ref() {
                    unsafe { ffi::VRT_RemoveVFP(ctx.raw, vp.as_ref()) }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

// Test issue 20 - null pointer drop
struct VFPTest {
    _buffer: Vec<u8>,
}

// Force a pass here to test to make sure that fini does not panic due to a null priv1 member
impl VFP for VFPTest {
    fn name() -> &'static CStr {
        c"vfptest"
    }

    fn new(_: &mut Ctx, _: &mut VFPCtx) -> InitResult<Self> {
        InitResult::Pass
    }

    fn pull(&mut self, _: &mut VFPCtx, _: &mut [u8]) -> PullResult {
        PullResult::Err
    }
}
