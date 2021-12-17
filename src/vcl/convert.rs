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
use varnish_sys::*;

/// Convert a Rust type into a VCL one
///
/// It will use the `WS` to persist the data during the VCL task if necessary
pub trait IntoVCL<T> {
    fn into_vcl(self, ws: &mut WS) -> T;
}

macro_rules! into_res {
    ( $x:ty ) => {
        impl IntoResult<&'static str> for $x {
            type Item = $x;
            fn into_result(self) -> Result<Self::Item, &'static str> {
                Ok(self)
            }
        }
    };
}

macro_rules! vcl_types {
    ($( $x:ident ),* $(,)?) => {
        $(
        impl IntoVCL<$x> for $x {
            fn into_vcl(self, _: &mut WS) -> $x {
                self
            }
        }
        into_res!($x);
        )*
    };
}

vcl_types!{
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
//    VCL_STRING, // same as VCL_BODY
    VCL_SUB,
//    VCL_TIME, // same as VCL_DURATION
    VCL_VCL,
//    VCL_VOID, // same as VCL_INSTANCE
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

impl IntoVCL<()> for () {
    fn into_vcl(self, _: &mut WS) {}
}

pub trait IntoResult<E> {
    type Item;
    fn into_result(self) -> Result<Self::Item, E>;
}

into_res!(());
into_res!(Duration);
into_res!(String);
into_res!(bool);

impl<'a> IntoResult<&'static str> for &'a str {
    type Item = &'a str;
    fn into_result(self) -> Result<Self::Item, &'static str> {
        Ok(self)
    }
}

impl<T, E: AsRef<str>> IntoResult<E> for Result<T, E> {
    type Item = T;
    fn into_result(self) -> Result<Self::Item, E> {
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

impl<T> IntoRust<VPriv<T>> for *mut vmod_priv {
    fn into_rust(self) -> VPriv<T> {
        VPriv::<T>::new(self)
    }
}
