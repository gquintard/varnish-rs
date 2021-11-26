use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::*;
use std::ptr;
use std::time::Duration;

use crate::vmod::vpriv::VPriv;
use crate::vrt::Ctx;
use varnish_sys;
use varnish_sys::{ VCL_REAL, VCL_INT, VCL_BOOL, VCL_STRING, VCL_DURATION };

pub trait IntoVCL<T> {
    fn into_vcl(self, ctx: &mut Ctx) -> T;
}

impl IntoVCL<VCL_REAL> for f64 {
    fn into_vcl(self, _: &mut Ctx) -> VCL_REAL {
        self.into()
    }
}

impl IntoVCL<VCL_INT> for i64 {
    fn into_vcl(self, _: &mut Ctx) -> VCL_INT {
        self as VCL_INT
    }
}

impl IntoVCL<VCL_BOOL> for bool {
    fn into_vcl(self, _: &mut Ctx) -> VCL_BOOL {
        self as VCL_BOOL
    }
}

impl IntoVCL<VCL_DURATION> for Duration {
    fn into_vcl(self, _: &mut Ctx) -> VCL_DURATION {
        self.as_secs_f64()
    }
}

impl IntoVCL<VCL_STRING> for &str {
    fn into_vcl(self, ctx: &mut Ctx) -> VCL_STRING {
        let l = self.len();
        match ctx.ws.alloc(l+1) {
            Err(_) => {
                ptr::null()
            },
            Ok(buf) => {
                buf[..l].copy_from_slice(self.as_bytes());
                buf[l] = '\0' as u8;
                buf.as_ptr() as *const i8
            }
        }
    }
}

impl IntoVCL<VCL_STRING> for String {
    fn into_vcl(self, ctx: &mut Ctx) -> VCL_STRING {
        <&str>::into_vcl(&self, ctx)
    }
}

impl IntoVCL<VCL_STRING> for &String {
    fn into_vcl(self, ctx: &mut Ctx) -> VCL_STRING {
        <&str>::into_vcl(&*self, ctx)
    }
}

impl IntoVCL<VCL_STRING> for VCL_STRING {
    fn into_vcl(self, _: &mut Ctx) -> VCL_STRING {
        self
    }
}

impl<T> IntoVCL<()> for T {
    fn into_vcl(self, _: &mut Ctx) -> () {}
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
