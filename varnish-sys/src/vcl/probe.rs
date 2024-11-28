use std::borrow::Cow;
use std::ffi::{c_char, c_uint, CStr};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::ffi::{vrt_backend_probe, VCL_DURATION, VCL_PROBE, VRT_BACKEND_PROBE_MAGIC};
use crate::vcl::{IntoVCL, VclError, Workspace};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Request<T> {
    URL(T),
    Text(T),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Probe<T = String> {
    pub request: Request<T>,
    pub timeout: Duration,
    pub interval: Duration,
    pub exp_status: c_uint,
    pub window: c_uint,
    pub threshold: c_uint,
    pub initial: c_uint,
}

pub type CowProbe<'a> = Probe<Cow<'a, str>>;

impl CowProbe<'_> {
    pub fn to_owned(&self) -> Probe {
        Probe {
            request: match &self.request {
                Request::URL(cow) => Request::URL(cow.to_string()),
                Request::Text(cow) => Request::Text(cow.to_string()),
            },
            timeout: self.timeout,
            interval: self.interval,
            exp_status: self.exp_status,
            window: self.window,
            threshold: self.threshold,
            initial: self.initial,
        }
    }
}

/// Helper to convert a probe into a VCL object
pub(crate) fn into_vcl_probe<T: AsRef<str>>(
    src: Probe<T>,
    ws: &mut Workspace,
) -> Result<VCL_PROBE, VclError> {
    let probe = ws.copy_value(vrt_backend_probe {
        magic: VRT_BACKEND_PROBE_MAGIC,
        timeout: src.timeout.into(),
        interval: src.interval.into(),
        exp_status: src.exp_status,
        window: src.window,
        initial: src.initial,
        ..Default::default()
    })?;

    match src.request {
        Request::URL(s) => {
            probe.url = s.as_ref().into_vcl(ws)?.0;
        }
        Request::Text(s) => {
            probe.request = s.as_ref().into_vcl(ws)?.0;
        }
    }

    Ok(VCL_PROBE(probe))
}

/// Helper to convert a VCL probe into a Rust probe wrapper
pub(crate) fn from_vcl_probe<'a, T: From<Cow<'a, str>>>(value: VCL_PROBE) -> Option<Probe<T>> {
    let pr = unsafe { value.0.as_ref()? };
    assert!(
        (pr.url.is_null() && !pr.request.is_null()) || pr.request.is_null() && !pr.url.is_null()
    );
    Some(Probe {
        request: if pr.url.is_null() {
            Request::Text(from_str(pr.request).into())
        } else {
            Request::URL(from_str(pr.url).into())
        },
        timeout: VCL_DURATION(pr.timeout).into(),
        interval: VCL_DURATION(pr.interval).into(),
        exp_status: pr.exp_status,
        window: pr.window,
        threshold: pr.threshold,
        initial: pr.initial,
    })
}

/// Helper function to convert a C string into a Rust string
fn from_str<'a>(value: *const c_char) -> Cow<'a, str> {
    if value.is_null() {
        Cow::Borrowed("")
    } else {
        // FIXME: this should NOT be lossy IMO
        unsafe { CStr::from_ptr(value).to_string_lossy() }
    }
}
