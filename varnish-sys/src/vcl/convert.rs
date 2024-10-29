//! Convert Rust types into their VCL_* equivalent, and back
//!
//! # Type conversion
//!
//! The proc macro will generate the wrappers for each user function, relying on
//! the type conversions defined here. The values need to be converted from Varnish's internal types
//! to Rust's types, and vice versa.
//!
//! Most conversions from VCL to Rust are straightforward, using either `From` or `TryFrom` traits.
//! The `IntoVCL` trait take care of converting a Rust type into VCL. It requires a `&mut `[`WS`]
//! to possibly store the returned value into the task request. This allows vmod writes to just return
//! easy-to-work-with strings, and let the boilerplate handle the allocation, copy and error handling.
//!
//! If one wants to handle things manually, all `VCL_*` types implement [`IntoVCL`] as a no-op. It
//! can be useful to avoid extra memory allocations by the boilerplate, if that is a worry.
//!
//! Here's a table of the type correspondences:
//!
//! | Rust | direction | VCL |
//! | :--: | :-------: | :-:
//! | `()` | -> | `VCL_VOID` |
//! | `f64`  | <-> | `VCL_REAL` |
//! | `i64`  | <-> | `VCL_INT` |
//! | `bool` | <-> | `VCL_BOOL` |
//! | `std::time::Duration` | <-> | `VCL_DURATION` |
//! | `&str` | <-> | `VCL_STRING` |
//! | `String` | -> | `VCL_STRING` |
//! | `Option<COWProbe>` | <-> | `VCL_PROBE` |
//! | `Option<Probe>` | <-> | `VCL_PROBE` |
//! | `Option<std::net::SockAdd>` | -> | `VCL_IP` |
//!
//! For all the other types, which are pointers, you will need to use the native types.
//!
//! *Note:* It is possible to simply return a `VCL_*` type (or a Result<VCL_*, _>), in which case
//! the boilerplate will just skip the conversion.
//!
//! # Result
//!
//! It's possible for a vmod writer to return a bare value, or a `Result<_, E: AsRef<str>>` to
//! potentially abort VCL processing in case the vmod hit an unrecoverable error.
//!
//! If a vmod function returns `Err(msg)`, the boilerplate will log `msg`, mark the current task as
//! failed and will return a default value to the VCL. In turn, the VCL will stop its processing
//! and will create a synthetic error object.

use std::borrow::Cow;
use std::ffi::{c_char, c_void, CStr};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::ptr::{null, null_mut};
use std::time::{Duration, SystemTime};

use crate::ffi::{
    sa_family_t, suckaddr, vrt_backend_probe, vsa_suckaddr_len, vtim_dur, vtim_real, VSA_BuildFAP,
    VSA_GetPtr, VSA_Port, PF_INET, PF_INET6, VCL_ACL, VCL_BACKEND, VCL_BLOB, VCL_BODY, VCL_BOOL,
    VCL_DURATION, VCL_ENUM, VCL_HEADER, VCL_HTTP, VCL_INT, VCL_IP, VCL_PROBE, VCL_REAL, VCL_REGEX,
    VCL_STEVEDORE, VCL_STRANDS, VCL_STRING, VCL_SUB, VCL_TIME, VCL_VCL, VRT_BACKEND_PROBE_MAGIC,
};
use crate::vcl::{COWProbe, COWRequest, Probe, Request, VclError, WS};

/// Convert a Rust type into a VCL one
///
/// It will use the [`WS`] to persist the data during the VCL task if necessary
pub trait IntoVCL<T> {
    fn into_vcl(self, ws: &mut WS) -> Result<T, VclError>;
}

macro_rules! default_null_ptr {
    ($ident:ident) => {
        default_null_ptr!($ident, null);
    };
    (mut $ident:ident) => {
        default_null_ptr!($ident, null_mut);
    };
    ($ident:ident, $func:ident) => {
        impl Default for $ident {
            fn default() -> Self {
                $ident($func())
            }
        }
    };
}

macro_rules! into_vcl_using_from {
    ($rust_ty:ty, $vcl_ty:ident) => {
        impl IntoVCL<$vcl_ty> for $rust_ty {
            fn into_vcl(self, _: &mut WS) -> Result<$vcl_ty, VclError> {
                Ok(self.into())
            }
        }
    };
}

macro_rules! from_rust_to_vcl {
    ($rust_ty:ty, $vcl_ty:ident) => {
        impl From<$rust_ty> for $vcl_ty {
            fn from(b: $rust_ty) -> Self {
                Self(b.into())
            }
        }
    };
}

macro_rules! from_vcl_to_rust {
    ($vcl_ty:ident, $rust_ty:ty) => {
        impl From<$vcl_ty> for $rust_ty {
            fn from(b: $vcl_ty) -> Self {
                <Self>::from(b.0)
            }
        }
    };
}

// VCL_ACL
default_null_ptr!(VCL_ACL);

// VCL_BACKEND
default_null_ptr!(VCL_BACKEND);

// VCL_BLOB
default_null_ptr!(VCL_BLOB);

// VCL_BODY
default_null_ptr!(VCL_BODY);

//
// VCL_BOOL
//
into_vcl_using_from!(bool, VCL_BOOL);
from_rust_to_vcl!(bool, VCL_BOOL);
impl From<VCL_BOOL> for bool {
    fn from(b: VCL_BOOL) -> Self {
        b.0 != 0
    }
}

//
// VCL_DURATION
//
into_vcl_using_from!(Duration, VCL_DURATION);
impl From<VCL_DURATION> for Duration {
    fn from(value: VCL_DURATION) -> Self {
        value.0.into()
    }
}
impl From<Duration> for VCL_DURATION {
    fn from(value: Duration) -> Self {
        Self(value.into())
    }
}

//
// vtim_dur -- this is a sub-structure of VCL_DURATION, equal to f64
//
impl From<vtim_dur> for Duration {
    fn from(value: vtim_dur) -> Self {
        Duration::from_secs_f64(value.0)
    }
}
impl From<Duration> for vtim_dur {
    fn from(value: Duration) -> Self {
        Self(value.as_secs_f64())
    }
}

// VCL_ENUM
default_null_ptr!(VCL_ENUM);
// VCL_HEADER
default_null_ptr!(VCL_HEADER);
// VCL_HTTP
default_null_ptr!(mut VCL_HTTP);

//
// VCL_INT
//
into_vcl_using_from!(i64, VCL_INT);
from_rust_to_vcl!(i64, VCL_INT);
from_vcl_to_rust!(VCL_INT, i64);

//
// VCL_IP
//
default_null_ptr!(VCL_IP);
impl IntoVCL<VCL_IP> for SocketAddr {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_IP, VclError> {
        unsafe {
            let p = ws.alloc(vsa_suckaddr_len)?.as_mut_ptr().cast::<suckaddr>();
            match self {
                SocketAddr::V4(sa) => {
                    assert!(!VSA_BuildFAP(
                        p.cast::<c_void>(),
                        PF_INET as sa_family_t,
                        sa.ip().octets().as_slice().as_ptr().cast::<c_void>(),
                        4,
                        (&sa.port().to_be() as *const u16).cast::<c_void>(),
                        2
                    )
                    .is_null());
                }
                SocketAddr::V6(sa) => {
                    assert!(!VSA_BuildFAP(
                        p.cast::<c_void>(),
                        PF_INET6 as sa_family_t,
                        sa.ip().octets().as_slice().as_ptr().cast::<c_void>(),
                        16,
                        (&sa.port().to_be() as *const u16).cast::<c_void>(),
                        2
                    )
                    .is_null());
                }
            }
            Ok(VCL_IP(p))
        }
    }
}
impl From<VCL_IP> for Option<SocketAddr> {
    fn from(value: VCL_IP) -> Self {
        let value = value.0;
        if value.is_null() {
            return None;
        }
        unsafe {
            let mut ptr = null();
            let fam = VSA_GetPtr(value, &mut ptr) as u32;
            let port = VSA_Port(value) as u16;

            match fam {
                PF_INET => {
                    let buf: &[u8; 4] = std::slice::from_raw_parts(ptr.cast::<u8>(), 4)
                        .try_into()
                        .unwrap();
                    Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(*buf)), port))
                }
                PF_INET6 => {
                    let buf: &[u8; 16] = std::slice::from_raw_parts(ptr.cast::<u8>(), 16)
                        .try_into()
                        .unwrap();
                    Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(*buf)), port))
                }
                _ => None,
            }
        }
    }
}

//
// VCL_PROBE
//
default_null_ptr!(VCL_PROBE);
impl<'a> IntoVCL<VCL_PROBE> for COWProbe<'a> {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_PROBE, VclError> {
        let p = ws
            .alloc(size_of::<vrt_backend_probe>())?
            .as_mut_ptr()
            .cast::<vrt_backend_probe>();
        let probe = unsafe { p.as_mut().unwrap() };
        probe.magic = VRT_BACKEND_PROBE_MAGIC;
        match self.request {
            COWRequest::URL(ref s) => {
                probe.url = s.into_vcl(ws)?.0;
            }
            COWRequest::Text(ref s) => {
                probe.request = s.into_vcl(ws)?.0;
            }
        }
        probe.timeout = self.timeout.into();
        probe.interval = self.interval.into();
        probe.exp_status = self.exp_status;
        probe.window = self.window;
        probe.initial = self.initial;
        Ok(VCL_PROBE(probe))
    }
}
impl IntoVCL<VCL_PROBE> for Probe {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_PROBE, VclError> {
        let p = ws
            .alloc(size_of::<vrt_backend_probe>())?
            .as_mut_ptr()
            .cast::<vrt_backend_probe>();
        let probe = unsafe { p.as_mut().unwrap() };
        probe.magic = VRT_BACKEND_PROBE_MAGIC;
        match self.request {
            Request::URL(ref s) => {
                probe.url = s.as_str().into_vcl(ws)?.0;
            }
            Request::Text(ref s) => {
                probe.request = s.as_str().into_vcl(ws)?.0;
            }
        }
        probe.timeout = self.timeout.into();
        probe.interval = self.interval.into();
        probe.exp_status = self.exp_status;
        probe.window = self.window;
        probe.initial = self.initial;
        Ok(VCL_PROBE(probe))
    }
}
impl<'a> From<VCL_PROBE> for Option<COWProbe<'a>> {
    fn from(value: VCL_PROBE) -> Self {
        let pr = unsafe { value.0.as_ref()? };
        assert!(
            (pr.url.is_null() && !pr.request.is_null())
                || pr.request.is_null() && !pr.url.is_null()
        );
        Some(COWProbe {
            request: if pr.url.is_null() {
                COWRequest::Text(from_str(pr.request))
            } else {
                COWRequest::URL(from_str(pr.url))
            },
            timeout: VCL_DURATION(pr.timeout).into(),
            interval: VCL_DURATION(pr.interval).into(),
            exp_status: pr.exp_status,
            window: pr.window,
            threshold: pr.threshold,
            initial: pr.initial,
        })
    }
}
impl From<VCL_PROBE> for Option<Probe> {
    fn from(value: VCL_PROBE) -> Self {
        let pr = unsafe { value.0.as_ref()? };
        assert!(
            (pr.url.is_null() && !pr.request.is_null())
                || pr.request.is_null() && !pr.url.is_null()
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
}

//
// VCL_REAL
//
into_vcl_using_from!(f64, VCL_REAL);
from_rust_to_vcl!(f64, VCL_REAL);
from_vcl_to_rust!(VCL_REAL, f64);

// VCL_REGEX
default_null_ptr!(VCL_REGEX);

//
// VCL_STRING
//
default_null_ptr!(VCL_STRING);
impl IntoVCL<VCL_STRING> for &[u8] {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, VclError> {
        // try to save some work if the buffer is already in the workspace
        // and if it ends in a null byte
        // FIXME: UB here - we check if the value AFTER the slice is a null byte
        //        in other words we access memory that is not ours. This is a bug.
        if unsafe { ws.is_slice_allocated(self) && *self.as_ptr().add(self.len()) == b'\0' } {
            Ok(VCL_STRING(self.as_ptr().cast::<c_char>()))
        } else {
            Ok(VCL_STRING(ws.copy_bytes_with_null(&self)?.as_ptr()))
        }
    }
}
impl IntoVCL<VCL_STRING> for &str {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, VclError> {
        self.as_bytes().into_vcl(ws)
    }
}
impl IntoVCL<VCL_STRING> for &Cow<'_, str> {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, VclError> {
        self.as_bytes().into_vcl(ws)
    }
}
impl IntoVCL<VCL_STRING> for String {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, VclError> {
        self.as_str().into_vcl(ws)
    }
}
impl<T: IntoVCL<VCL_STRING> + AsRef<[u8]>> IntoVCL<VCL_STRING> for Option<T> {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, VclError> {
        match self {
            None => Ok(VCL_STRING(null())),
            Some(t) => t.as_ref().into_vcl(ws),
        }
    }
}
impl<'a> From<VCL_STRING> for Cow<'a, str> {
    fn from(value: VCL_STRING) -> Self {
        from_str(value.0)
    }
}

/// Helper function
fn from_str<'a>(value: *const c_char) -> Cow<'a, str> {
    if value.is_null() {
        Cow::Borrowed("")
    } else {
        // FIXME: this should NOT be lossy IMO
        unsafe { CStr::from_ptr(value).to_string_lossy() }
    }
}

// VCL_STEVEDORE
default_null_ptr!(VCL_STEVEDORE);
// VCL_STRANDS
default_null_ptr!(VCL_STRANDS);
// VCL_SUB
default_null_ptr!(VCL_SUB);

//
// VCL_TIME
//
impl IntoVCL<VCL_TIME> for SystemTime {
    fn into_vcl(self, _: &mut WS) -> Result<VCL_TIME, VclError> {
        self.try_into()
    }
}
impl TryFrom<SystemTime> for VCL_TIME {
    type Error = VclError;

    fn try_from(value: SystemTime) -> Result<Self, Self::Error> {
        Ok(VCL_TIME(vtim_real(
            value
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_err(|e| VclError::new(e.to_string()))?
                .as_secs_f64(),
        )))
    }
}

// VCL_VCL
default_null_ptr!(mut VCL_VCL);
