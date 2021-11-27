use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::*;
use std::ptr;
use std::time::Duration;

use crate::vmod::vpriv::VPriv;
use crate::vrt::WS;
use varnish_sys;
use varnish_sys::{VCL_BOOL, VCL_DURATION, VCL_INT, VCL_REAL, VCL_STRING};

pub trait IntoVCL<T> {
    fn into_vcl(self, ws: &mut WS) -> T;
}

impl IntoVCL<VCL_REAL> for f64 {
    fn into_vcl(self, _: &mut WS) -> VCL_REAL {
        self
    }
}

impl IntoVCL<VCL_INT> for i64 {
    fn into_vcl(self, _: &mut WS) -> VCL_INT {
        self as VCL_INT
    }
}

impl IntoVCL<VCL_BOOL> for bool {
    fn into_vcl(self, _: &mut WS) -> VCL_BOOL {
        self as VCL_BOOL
    }
}

impl IntoVCL<VCL_DURATION> for Duration {
    fn into_vcl(self, _: &mut WS) -> VCL_DURATION {
        self.as_secs_f64()
    }
}

impl IntoVCL<VCL_STRING> for &str {
    fn into_vcl(self, ws: &mut WS) -> VCL_STRING {
        let l = self.len();
        match ws.alloc(l + 1) {
            Err(_) => ptr::null(),
            Ok(buf) => {
                buf[..l].copy_from_slice(self.as_bytes());
                buf[l] = b'\0';
                buf.as_ptr() as *const i8
            }
        }
    }
}

impl IntoVCL<VCL_STRING> for String {
    fn into_vcl(self, ws: &mut WS) -> VCL_STRING {
        <&str>::into_vcl(&self, ws)
    }
}

impl IntoVCL<VCL_STRING> for VCL_STRING {
    fn into_vcl(self, _: &mut WS) -> VCL_STRING {
        self
    }
}

impl IntoVCL<()> for () {
    fn into_vcl(self, _: &mut WS) {}
}

const EMPTY_STRING: *const c_char = b"\0".as_ptr() as *const c_char;

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

impl<T> IntoRust<VPriv<T>> for *mut varnish_sys::vmod_priv {
    fn into_rust(self) -> VPriv<T> {
        VPriv::<T>::new(self)
    }
}
