//! Convert Rust types into their VCL_* equivalent, and back
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
//! If one wants to hand manually, `VCL_STRING` to `VCL_STRING` is implemented as a no-op, allowing
//! the vmod writer to do the work manually if wished.
//!
//! Here's a table of the type correspondences:
//!
//! | Rust | direction | VCL |
//! | :--: | :-------: | :-:
//! | `f64`  | <-> | `VCL_REAL` |
//! | `i64`  | <-> | `VCL_INT` |
//! | `bool` | <-> | `VCL_BOOL` |
//! | `std::time::Duration` | <-> | `VCL_DURATION` |
//! | `()` | <-> | `VOID` |
//! | `&str` | <-> | `VCL_STRING` |
//! | `String` | -> | `VCL_STRING` |
//! | `VCL_STRING` | -> | `VCL_STRING` |
//!
//!
use std::borrow::Cow;
use std::ffi::CStr;
use std::os::raw::*;
use std::ptr;
use std::time::Duration;

use crate::vcl::vpriv::VPriv;
use crate::vcl::ws::WS;
use varnish_sys::{VCL_BOOL, VCL_DURATION, VCL_INT, VCL_REAL, VCL_STRING};

/// Convert a Rust type into a VCL one
///
/// It will use the `WS` to persist the data during the VCL task if necessary
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

pub trait IntoResult<U> {
    type Item;
    fn into_result(self) -> Result<Self::Item, U>;
}

impl<T: IntoVCL> IntoResult<&'static str> for T {
    type Item = T;
    fn into_result(self) -> Result<Self::Item, &'static str> {
        Ok(self)
    }
}

impl<T, U: AsRef<str>> IntoResult<U> for Result<T, U> {
    type Item = T;
    fn into_result(self) -> Result<Self::Item, U> {
        self
    }
}

const EMPTY_STRING: *const c_char = b"\0".as_ptr() as *const c_char;

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

impl<T> IntoRust<VPriv<T>> for *mut varnish_sys::vmod_priv {
    fn into_rust(self) -> VPriv<T> {
        VPriv::<T>::new(self)
    }
}
