use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::*;
use std::ptr;
use std::time::Duration;

use crate::vcl::vpriv::VPriv;
use crate::vcl::ws::WS;
use varnish_sys::{VCL_BOOL, VCL_DURATION, VCL_INT, VCL_REAL, VCL_STRING};

pub trait IntoVCL {
    type Item;
    fn into_vcl(self, ws: &mut WS) -> Self::Item;
}

impl IntoVCL for f64 {
    type Item = VCL_REAL;
    fn into_vcl(self, _: &mut WS) -> Self::Item {
        self
    }
}

impl IntoVCL for i64 {
    type Item = VCL_INT;
    fn into_vcl(self, _: &mut WS) -> Self::Item {
        self as VCL_INT
    }
}

impl IntoVCL for bool {
    type Item = VCL_BOOL;
    fn into_vcl(self, _: &mut WS) -> Self::Item {
        self as VCL_BOOL
    }
}

impl IntoVCL for Duration {
    type Item = VCL_DURATION;
    fn into_vcl(self, _: &mut WS) -> Self::Item {
        self.as_secs_f64()
    }
}

impl IntoVCL for &str {
    type Item = VCL_STRING;
    fn into_vcl(self, ws: &mut WS) -> Self::Item {
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

impl IntoVCL for String {
    type Item = VCL_STRING;
    fn into_vcl(self, ws: &mut WS) -> Self::Item {
        <&str>::into_vcl(&self, ws)
    }
}

impl IntoVCL for VCL_STRING {
    type Item = VCL_STRING;
    fn into_vcl(self, _: &mut WS) -> Self::Item {
        self
    }
}

impl IntoVCL for () {
    type Item = ();
    fn into_vcl(self, _: &mut WS) -> Self::Item {}
}

pub trait IntoResult {
    type Item;
    fn into_result(self) -> Result<Self::Item, String>;
}

impl<T: IntoVCL> IntoResult for T {
    type Item = T;
    fn into_result(self) -> Result<Self::Item, String> {
        Ok(self)
    }
}

impl<T> IntoResult for Result<T, String> {
    type Item = T;
    fn into_result(self) -> Result<Self::Item, String> {
        self
    }
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
