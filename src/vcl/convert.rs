//! Convert Rust types into their VCL_* equivalent, and back
//!
//! # Type conversion
//!
//! To allow for easier development the generated boilerplate will handle conversion between the
//! lightly disguised C types used by `vmod.vcc` into regular Rust, and it will also do the
//! opposite conversion when it is time to send the return value to Varnish.
//!
//! The two traits `IntoVCL` and `IntoRust` take care of this, with `IntoVCL` being notable in
//! that it requires a `&mut `[`crate::vcl::ws::WS`] to possibly store the returned value into the task
//! request. This allows vmod writes to just return easy-to-work-with `Strings` and let the
//! boilerplate handle the allocation, copy and error handling.
//!
//! If one wants to handle things manually, all `VCL_*` types implement [`IntoVCL`] as a no-op. It
//! can be useful to avoid extra memory allocations by the boilerplate, if that is a worry.
//!
//! Here's a table of the type correspondences:
//!
//! | Rust | direction | VCL |
//! | :--: | :-------: | :-:
//! | `f64`  | <-> | `VCL_REAL` |
//! | `i64`  | <-> | `VCL_INT` |
//! | `i64`  | <-> | `VCL_BYTES` |
//! | `bool` | <-> | `VCL_BOOL` |
//! | `std::time::Duration` | <-> | `VCL_DURATION` |
//! | `()` | <-> | `VOID` |
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
//! If a vmod function returns `Err(msg)`, the boilerplat will log `msg`, marke the current task as
//! failed and will return a default value to the VCL. In turn, the VCL will stop its processing
//! and will create a synthetic error object.
use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::*;
use std::ptr;
use std::time::Duration;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::vcl::vpriv::VPriv;
use crate::vcl::ws::WS;
use crate::vcl::probe;
use crate::vcl::probe::{ COWProbe, Probe };
use varnish_sys::*;

/// Convert a Rust type into a VCL one
///
/// It will use the `WS` to persist the data during the VCL task if necessary
pub trait IntoVCL<T> {
    fn into_vcl(self, ws: &mut WS) -> Result<T, String>;
}

macro_rules! into_res {
    ( $x:ty ) => {
        impl IntoResult<String> for $x {
            type Item = $x;
            fn into_result(self) -> Result<Self::Item, String> {
                Ok(self)
            }
        }
        impl<E: ToString> IntoResult<E> for Result<$x, E> {
            type Item = $x;
            fn into_result(self) -> Result<Self::Item, String> {
                self.map_err(|x| x.to_string())
            }
        }
    };
}

macro_rules! vcl_types {
    ($( $x:ident ),* $(,)?) => {
        $(
        impl IntoVCL<$x> for $x {
            fn into_vcl(self, _: &mut WS) -> Result<$x, String> {
                Ok(self)
            }
        }
        into_res!($x);
        )*
    };
}

vcl_types! {
    VCL_ACL,
    VCL_BACKEND,
    VCL_BLOB,
    VCL_BODY,
    VCL_BOOL,
    VCL_BYTES,
    VCL_DURATION,
//    VCL_ENUM, // same as VCL_BODY
    VCL_HEADER,
    VCL_HTTP,
    VCL_INSTANCE,
//    VCL_INT, // same as VCL_BYTES
    VCL_IP,
    VCL_PROBE,
//    VCL_REAL, // same as VCL_DURATION
    VCL_REGEX,
    VCL_STEVEDORE,
    VCL_STRANDS,
    VCL_STRING,
    VCL_SUB,
//    VCL_TIME, // same as VCL_DURATION
    VCL_VCL,
//    VCL_VOID, // same as VCL_INSTANCE
}

impl IntoVCL<()> for () {
    fn into_vcl(self, _: &mut WS) -> Result<(), String> {
        Ok(())
    }
}

impl IntoVCL<VCL_BOOL> for bool {
    fn into_vcl(self, _: &mut WS) -> Result<VCL_BOOL, String> {
        Ok(self as VCL_BOOL)
    }
}

impl IntoVCL<VCL_DURATION> for Duration {
    fn into_vcl(self, _: &mut WS) -> Result<VCL_DURATION, String> {
        Ok(self.as_secs_f64())
    }
}

impl IntoVCL<VCL_STRING> for &[u8] {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, String> {
        // try to save some work if the buffer is already in the workspace
        // and if it's followed by a null byten
        if unsafe { varnish_sys::WS_Allocated(ws.raw, self.as_ptr() as *const c_void, self.len() as isize + 1) == 1 && *self.as_ptr().add(self.len()) == b'\0' } {
            Ok(self.as_ptr() as *const c_char)
        } else {
            Ok(ws.copy_bytes_with_null(&self)?.as_ptr() as *const c_char)
        }
    }
}

impl IntoVCL<VCL_STRING> for &str {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, String> {
        self.as_bytes().into_vcl(ws)
    }
}

impl IntoVCL<VCL_STRING> for String {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, String> {
        self.as_str().into_vcl(ws)
    }
}

impl IntoVCL<()> for Result<(), String> {
    fn into_vcl(self, _: &mut WS) -> Result<(), String> {
        Ok(())
    }
}

impl<T: IntoVCL<VCL_STRING> + AsRef<[u8]>> IntoVCL<VCL_STRING> for Option<T> {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_STRING, String> {
        match self {
            None => Ok(ptr::null()),
            Some(t) => t.as_ref().into_vcl(ws),
        }
    }
}

impl<'a> IntoVCL<VCL_PROBE> for COWProbe<'a> {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_PROBE, String> {
        let p = ws.alloc(std::mem::size_of::<varnish_sys::vrt_backend_probe>())?.as_mut_ptr() as *mut vrt_backend_probe;
        let probe = unsafe { p.as_mut().unwrap() };
        probe.magic = varnish_sys::VRT_BACKEND_PROBE_MAGIC;
        match self.request {
            probe::COWRequest::URL(ref s) => { probe.url = s.into_vcl(ws)?; },
            probe::COWRequest::Text(ref s) => { probe.request = s.into_vcl(ws)?; },
        }
        probe.timeout = self.timeout.into_vcl(ws)?;
        probe.interval = self.interval.into_vcl(ws)?;
        probe.exp_status = self.exp_status.into_vcl(ws)?;
        probe.window = self.window.into_vcl(ws)?;
        probe.initial = self.initial.into_vcl(ws)?;
        Ok(probe)
    }
}

impl IntoVCL<VCL_PROBE> for Probe {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_PROBE, String> {
        let p = ws.alloc(std::mem::size_of::<varnish_sys::vrt_backend_probe>())?.as_mut_ptr() as *mut vrt_backend_probe;
        let probe = unsafe { p.as_mut().unwrap() };
        probe.magic = varnish_sys::VRT_BACKEND_PROBE_MAGIC;
        match self.request {
            probe::Request::URL(ref s) => { probe.url = s.as_str().into_vcl(ws)?; },
            probe::Request::Text(ref s) => { probe.request = s.as_str().into_vcl(ws)?; },
        }
        probe.timeout = self.timeout.into_vcl(ws)?;
        probe.interval = self.interval.into_vcl(ws)?;
        probe.exp_status = self.exp_status.into_vcl(ws)?;
        probe.window = self.window.into_vcl(ws)?;
        probe.initial = self.initial.into_vcl(ws)?;
        Ok(probe)
    }
}

impl IntoVCL<VCL_IP> for SocketAddr {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_IP, String> {
        unsafe{
            let p = ws.alloc(varnish_sys::vsa_suckaddr_len)?.as_mut_ptr() as *mut varnish_sys::suckaddr;
            match self {
                std::net::SocketAddr::V4(sa) => {
                    assert!(!varnish_sys::VSA_BuildFAP(
                            p as *mut std::ffi::c_void,
                            varnish_sys::PF_INET as varnish_sys::sa_family_t,
                            sa.ip().octets().as_slice().as_ptr() as *const std::os::raw::c_void,
                            4,
                            &sa.port().to_be() as *const u16 as *const std::os::raw::c_void,
                            2
                            ).is_null());
                },
                std::net::SocketAddr::V6(sa) => {
                    assert!(!varnish_sys::VSA_BuildFAP(
                            p as *mut std::ffi::c_void,
                            varnish_sys::PF_INET6 as varnish_sys::sa_family_t,
                            sa.ip().octets().as_slice().as_ptr() as *const std::os::raw::c_void,
                            16,
                            &sa.port().to_be() as *const u16 as *const std::os::raw::c_void,
                            2
                            ).is_null());
                },
            }
            Ok(p)
        }
    }
}

impl IntoVCL<VCL_IP> for Option<SocketAddr> {
    fn into_vcl(self, ws: &mut WS) -> Result<VCL_IP, String> {
        match self {
            None => Ok(std::ptr::null()),
            Some(ip) => ip.into_vcl(ws),
        }
    }
}

/// Create a `Result` from a bare value, or from a `Result`
///
/// For code simplicity, the boilerplate expects all vmod functions to return a `Result`, and for
/// ease-of-use, vmod functions can return either `T: IntoVCL` or `Result<T: IntoVCL, E: AsRef<str>>.
/// `into_result` is in charge of the normalization.
pub trait IntoResult<E> {
    type Item;
    fn into_result(self) -> Result<Self::Item, String>;
}

into_res!(());
into_res!(Duration);
into_res!(String);
into_res!(bool);
into_res!(Option<String>);
into_res!(SocketAddr);

impl<'a> IntoResult<String> for COWProbe<'a> {
    type Item = COWProbe<'a>;
    fn into_result(self) -> Result<Self::Item, String> {
        Ok(self)
    }
}

impl IntoResult<String> for Probe {
    type Item = Probe;
    fn into_result(self) -> Result<Self::Item, String> {
        Ok(self)
    }
}

impl<'a, E: ToString> IntoResult<E> for Result<COWProbe<'a>, E> {
    type Item = COWProbe<'a>;
    fn into_result(self) -> Result<Self::Item, String> {
        self.map_err(|x| x.to_string())
    }
}

impl<E: ToString> IntoResult<E> for Result<Probe, E> {
    type Item = Probe;
    fn into_result(self) -> Result<Self::Item, String> {
        self.map_err(|x| x.to_string())
    }
}

impl<'a> IntoResult<String> for &'a str {
    type Item = Self;
    fn into_result(self) -> Result<Self::Item, String> {
        Ok(self)
    }
}

impl<'a, E: ToString> IntoResult<E> for Result<&'a str, E> {
    type Item = &'a str;
    fn into_result(self) -> Result<Self::Item, String> {
        self.map_err(|x| x.to_string())
    }
}

impl<'a> IntoResult<String> for Option<&'a str> {
    type Item = Self;
    fn into_result(self) -> Result<Self::Item, String> {
        Ok(self)
    }
}

impl<'a, E: ToString> IntoResult<E> for Result<Option<&'a str>, E> {
    type Item = Option<&'a str>;
    fn into_result(self) -> Result<Self::Item, String> {
        self.map_err(|x| x.to_string())
    }
}

impl<'a> IntoResult<String> for Option<&'a [u8]> {
    type Item = Self;
    fn into_result(self) -> Result<Self::Item, String> {
        Ok(self)
    }
}

impl<'a, E: ToString> IntoResult<E> for Result<&'a [u8], E> {
    type Item = &'a [u8];
    fn into_result(self) -> Result<Self::Item, String> {
        self.map_err(|x| x.to_string())
    }
}

impl<'a, E: ToString> IntoResult<E> for Result<Option<&'a [u8]>, E> {
    type Item = Option<&'a [u8]>;
    fn into_result(self) -> Result<Self::Item, String> {
        self.map_err(|x| x.to_string())
    }
}


pub trait VCLDefault {
    type Item;
    fn vcl_default() -> Self::Item;
}

/// Generate a default value to return.
///
/// `Default` isn't implemented for `std::ptr`, so we roll out our own.
impl<T> VCLDefault for *const T {
    type Item = *const T;
    fn vcl_default() -> Self::Item {
        ptr::null()
    }
}

impl VCLDefault for f64 {
    type Item = f64;
    fn vcl_default() -> Self::Item {
        0.0
    }
}

impl VCLDefault for i64 {
    type Item = i64;
    fn vcl_default() -> Self::Item {
        0
    }
}

impl VCLDefault for u32 {
    type Item = u32;
    fn vcl_default() -> Self::Item {
        0
    }
}

impl VCLDefault for () {
    type Item = ();
    fn vcl_default() -> Self::Item {}
}

const EMPTY_STRING: *const c_char = c"".as_ptr();

/// Convert a VCL type into a Rust one.
///
/// Note that for buffer-based types (only `VCL_STRING` at the moment), the lifetimes are not tied
/// to the `Ctx` for simplicity. It may change in the future, but for now, the caller must ensure
/// that the Rust object doesn't outlive the C object as it doesn't copy it but merely points at
/// it.
pub trait IntoRust<T> {
    fn into_rust(self) -> T;
}

impl IntoRust<f64> for VCL_REAL {
    fn into_rust(self) -> f64 {
        self as f64
    }
}

impl IntoRust<i64> for VCL_INT {
    fn into_rust(self) -> i64 {
        self as i64
    }
}

impl IntoRust<bool> for VCL_BOOL {
    fn into_rust(self) -> bool {
        self != 0
    }
}

impl<'a> IntoRust<Cow<'a, str>> for VCL_STRING {
    fn into_rust(self) -> Cow<'a, str> {
        let s = if self.is_null() { EMPTY_STRING } else { self };
        unsafe { CStr::from_ptr(s).to_string_lossy() }
    }
}

impl IntoRust<Duration> for VCL_DURATION {
    fn into_rust(self) -> Duration {
        Duration::from_secs_f64(self as f64)
    }
}

impl<T> IntoRust<VPriv<T>> for *mut vmod_priv {
    fn into_rust(self) -> VPriv<T> {
        VPriv::<T>::new(self)
    }
}

impl<'a> IntoRust<Option<COWProbe<'a>>> for VCL_PROBE {
    fn into_rust(self) -> Option<COWProbe<'a>> {
        let pr = unsafe { self.as_ref()? };
        assert!((pr.url.is_null() && !pr.request.is_null()) || pr.request.is_null() && !pr.url.is_null());
        Some(COWProbe {
            request: if !pr.url.is_null() {
                crate::vcl::probe::COWRequest::URL(pr.url.into_rust())
            } else {
                crate::vcl::probe::COWRequest::Text(pr.request.into_rust())
            },
            timeout: pr.timeout.into_rust(),
            interval: pr.interval.into_rust(),
            exp_status: pr.exp_status,
            window: pr.window,
            threshold: pr.threshold,
            initial: pr.initial,
        })
    }
}

impl IntoRust<Option<Probe>> for VCL_PROBE {
    fn into_rust(self) -> Option<Probe> {
        let pr = unsafe { self.as_ref()? };
        assert!((pr.url.is_null() && !pr.request.is_null()) || pr.request.is_null() && !pr.url.is_null());
        Some(Probe {
            request: if !pr.url.is_null() {
                probe::Request::URL(pr.url.into_rust().to_string())
            } else {
                probe::Request::Text(pr.request.into_rust().to_string())
            },
            timeout: pr.timeout.into_rust(),
            interval: pr.interval.into_rust(),
            exp_status: pr.exp_status,
            window: pr.window,
            threshold: pr.threshold,
            initial: pr.initial,
        })
    }
}

impl IntoRust<Option<SocketAddr>> for VCL_IP {
    fn into_rust(self) -> Option<SocketAddr> {
        unsafe {
            if self.is_null() {
                return None;
            }
            let mut ptr = std::ptr::null();
            let fam = varnish_sys::VSA_GetPtr(self, &mut ptr) as u32;
            let port = VSA_Port(self) as u16;

            match fam {
                varnish_sys::PF_INET => {
                    let buf: &[u8; 4] = std::slice::from_raw_parts(ptr as *const u8, 4).try_into().unwrap();
                    Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::from(*buf)), port))
                },
                varnish_sys::PF_INET6 => {
                    let buf: &[u8; 16] = std::slice::from_raw_parts(ptr as *const u8, 16).try_into().unwrap();
                    Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::from(*buf)), port))
                },
                _ => {
                    None
                },
            }
        }
    }
}
