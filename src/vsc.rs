//! Access Varnish statistics
//!
//! The VSC (Varnish Shared Counter) is a way for outside program to access Varnish statistics in a
//! non-blocking way. The main way to access those counters traditionally is with `varnishstat`,
//! but the API is generic and allows you to track, filter and read any counter that `varnishd`
//! (and vmods) are exposing.

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString, NulError};
use std::marker::PhantomData;
use std::path::Path;
use std::ptr;
use std::time::Duration;

pub use crate::error::{Error, Result};
use crate::ffi;

/// A statistics set, created using a [`VSCBuilder`]
#[derive(Debug)]
pub struct VSC<'a> {
    vsm: *mut ffi::vsm,
    vsc: *mut ffi::vsc,
    internal: Box<VSCInternal<'a>>,
}

#[derive(Debug, Default)]
struct VSCInternal<'a> {
    points: HashMap<usize, Stat<'a>>,
    added: Vec<usize>,
    deleted: Vec<usize>,
}

/// Initialize and configure a [`VSC`] but do not attach it to a running `varnishd` instance
pub struct VSCBuilder<'a> {
    vsm: *mut ffi::vsm,
    vsc: *mut ffi::vsc,
    phantom: PhantomData<&'a ()>,
}

impl<'a> VSCBuilder<'a> {
    /// Create a new `VSCBuilder`
    pub fn new() -> Self {
        unsafe {
            let vsm = ffi::VSM_New();
            assert!(!vsm.is_null());
            let vsc = ffi::VSC_New();
            assert!(!vsc.is_null());
            // get raw value, we can always clamp them later
            ffi::VSC_Arg(vsc, 'r' as c_char, ptr::null());
            VSCBuilder {
                vsm,
                vsc,
                phantom: PhantomData,
            }
        }
    }

    /// Specify where to find the `varnishd` working directory.
    ///
    /// It's usually superfluous to call this function, unless `varnishd` itself was called with
    /// the `-n` argument (in which case, both arguments should match)
    pub fn work_dir(self, dir: &Path) -> std::result::Result<Self, NulError> {
        let c_dir = CString::new(dir.to_str().unwrap())?;
        let ret = unsafe { ffi::VSM_Arg(self.vsm, 'n' as c_char, c_dir.as_ptr()) };
        assert_eq!(ret, 1);
        Ok(self)
    }

    /// How long to wait when attaching
    ///
    /// When [`VSCBuilder::build()`] is called, it'll internally call `VSM_Attach`, hoping to find a running
    /// `varnishd` instance. If `None`, the function will not return until it connects, otherwise
    /// it specifies the timeout to use.
    pub fn patience(self, t: Option<Duration>) -> Result<Self> {
        // the things we do for love...
        let arg = match t {
            None => "off".to_string(),
            Some(t) => format!("{}\0", t.as_secs_f64()),
        };
        unsafe {
            let ret = ffi::VSM_Arg(self.vsm, 't' as c_char, arg.as_ptr().cast::<c_char>());
            assert_eq!(ret, 1);
        }
        Ok(self)
    }

    fn vsc_arg(self, o: char, s: &str) -> std::result::Result<Self, NulError> {
        let c_s = CString::new(s)?;
        unsafe {
            let ret = ffi::VSC_Arg(self.vsc, o as c_char, c_s.as_ptr().cast::<c_char>());
            assert_eq!(ret, 1);
        }
        Ok(self)
    }

    /// Provide a globbing pattern of statistics names to include.
    ///
    /// May be called multiple times, interleaved with [`VSCBuilder::exclude()`], the order matters.
    pub fn include(self, s: &str) -> std::result::Result<Self, NulError> {
        self.vsc_arg('I', s)
    }

    /// Provide a globbing pattern of statistics names to exclude.
    ///
    /// May be called multiple times, interleaved with [`VSCBuilder::include()`], the order matters.
    pub fn exclude(self, s: &str) -> std::result::Result<Self, NulError> {
        self.vsc_arg('X', s)
    }

    /// Provide a globbing pattern of statistics names to keep around, protecting them from
    /// exclusion.
    pub fn require(self, s: &str) -> std::result::Result<Self, NulError> {
        self.vsc_arg('R', s)
    }

    /// Build the [`VSC`], attaching to a running `varnishd` instance
    pub fn build(mut self) -> Result<VSC<'a>> {
        let ret = unsafe { ffi::VSM_Attach(self.vsm, 0) };
        if ret != 0 {
            let err = vsm_error(self.vsm);
            unsafe {
                ffi::VSM_ResetError(self.vsm);
            }
            Err(err)
        } else {
            let mut internal = Box::new(VSCInternal::default());
            unsafe {
                ffi::VSC_State(
                    self.vsc,
                    Some(add_point),
                    Some(del_point),
                    (&mut *internal as *mut VSCInternal).cast::<c_void>(),
                );
            }
            let vsm = self.vsm;
            let vsc = self.vsc;
            // nullify so that .drop() doesn't destroy vsm/vsc
            self.vsm = ptr::null_mut();
            self.vsc = ptr::null_mut();
            Ok(VSC { vsm, vsc, internal })
        }
    }
}

fn vsm_error(p: *const ffi::vsm) -> Error {
    unsafe {
        Error::new(
            CStr::from_ptr(ffi::VSM_Error(p))
                .to_str()
                .unwrap()
                .to_string(),
        )
    }
}

impl<'a> Drop for VSCBuilder<'a> {
    fn drop(&mut self) {
        assert!(
            (self.vsc.is_null() && self.vsm.is_null())
                || (!self.vsc.is_null() && !self.vsm.is_null())
        );
        if !self.vsc.is_null() {
            unsafe {
                ffi::VSC_Destroy(&mut self.vsc, self.vsm);
            }
        }
    }
}

impl<'a> Drop for VSC<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::VSC_Destroy(&mut self.vsc, self.vsm);
        }
    }
}

/// Kind of data
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Semantics {
    /// Value only goes up (e.g. amount of bytes transfered)
    Counter,
    /// Value can go up and down (e.g. amount of current connections)
    Gauge,
    /// Value is to be read as 64 booleans packed together as a `u64`
    Bitmap,
    /// No information on this value
    Unknown,
}

impl From<c_int> for Semantics {
    fn from(value: c_int) -> Self {
        let c = char::from_u32(value as u32).unwrap();
        match c {
            'c' => Semantics::Counter,
            'g' => Semantics::Gauge,
            'b' => Semantics::Bitmap,
            _ => Semantics::Unknown,
        }
    }
}

impl From<Semantics> for char {
    fn from(value: Semantics) -> char {
        match value {
            Semantics::Counter => 'c',
            Semantics::Gauge => 'g',
            Semantics::Bitmap => 'b',
            Semantics::Unknown => '?',
        }
    }
}

/// Unit of a value
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Format {
    /// No unit
    Integer,
    /// Bytes, for data volumes
    Bytes,
    /// No unit, generally used with [`Semantics::Bitmap`]
    Bitmap,
    /// Time
    Duration,
    /// Unit unknown
    Unknown,
}

impl From<c_int> for Format {
    fn from(value: c_int) -> Self {
        let c = char::from_u32(value as u32).unwrap();
        match c {
            'i' => Format::Integer,
            'B' => Format::Bytes,
            'b' => Format::Bitmap,
            'd' => Format::Duration,
            _ => Format::Unknown,
        }
    }
}

impl From<Format> for char {
    fn from(value: Format) -> char {
        match value {
            Format::Integer => 'i',
            Format::Bytes => 'B',
            Format::Bitmap => 'b',
            Format::Duration => 'd',
            Format::Unknown => '?',
        }
    }
}

unsafe extern "C" fn add_point(ptr: *mut c_void, point: *const ffi::VSC_point) -> *mut c_void {
    let internal = ptr.cast::<VSCInternal>();
    let k = point as usize;

    let stat = Stat {
        value: (*point).ptr,
        name: CStr::from_ptr((*point).name).to_str().unwrap(),
        short_desc: CStr::from_ptr((*point).sdesc).to_str().unwrap(),
        long_desc: CStr::from_ptr((*point).ldesc).to_str().unwrap(),
        semantics: (*point).semantics.into(),
        format: (*point).format.into(),
    };
    assert_eq!((*internal).points.insert(k, stat), None);
    (*internal).added.push(k);
    ptr::null_mut()
}

unsafe extern "C" fn del_point(ptr: *mut c_void, point: *const ffi::VSC_point) {
    let internal = ptr.cast::<VSCInternal>();
    let k = point as usize;
    assert!((*internal).points.contains_key(&k));

    (*internal).deleted.push(k);
    assert!((*internal).points.remove(&k).is_some());
}

/// A live statistic
///
/// Describes a live value, with little overhead over the C struct
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Stat<'a> {
    value: *const u64,
    pub name: &'a str,
    pub short_desc: &'a str,
    pub long_desc: &'a str,
    pub semantics: Semantics,
    pub format: Format,
}

impl<'a> Stat<'a> {
    /// Retrieve the current value of the statistic, as-is
    pub fn get_raw_value(&self) -> u64 {
        // # Safety
        // the pointer is valid as long as the VSC exist, and
        // VSC.update() isn't called (which uses `&mut self`)
        unsafe { *self.value }
    }

    /// Return a sanitized value of the statistic
    ///
    /// Gauges will fluctuate up and down, with multiple threads operating on them. As a result,
    /// their value can go below 0 and underflow. This function will prevent the value from
    /// wrapping around and just returns 0.
    pub fn get_clamped_value(&self) -> u64 {
        // # Safety
        // the pointer is valid as long as VSC exist, and
        // VSC.update() isn't called (which uses `&mut self`)
        let v = unsafe { *self.value };
        if v <= i64::MAX as u64 {
            v
        } else {
            0
        }
    }
}

impl<'a> VSC<'a> {
    /// Return a statistic set
    ///
    /// Names are not necessarily unique, so instead, statistics are tracked using `usize` handle
    /// that can help you track which ones (dis)appeared during a [`VSC::update()`] call.
    ///
    /// The C API guarantees we can access all the `Stat` in the `HashMap`, until the next `update`
    /// call, so the `rust` API mirrors this here.
    pub fn stats(&self) -> &HashMap<usize, Stat> {
        &self.internal.points
    }

    /// Update the list of `Stat` we have access to
    ///
    /// You must call this function at least once to get access to any data (otherwise you'll just
    /// get an empty `HashMap`).
    ///
    /// The two returned `Vec`s list the added and deleted keys in the `HashMap`, in case you need
    /// to keep track of them at an individual level.
    /// (if a key appears in both `Vec`s, the statistic got replaced).
    pub fn update(&mut self) -> (Vec<usize>, Vec<usize>) {
        unsafe {
            ffi::VSC_Iter(self.vsc, self.vsm, None, ptr::null_mut());
        }
        let added = std::mem::take(&mut self.internal.added);
        let deleted = std::mem::take(&mut self.internal.deleted);
        (added, deleted)
    }
}
