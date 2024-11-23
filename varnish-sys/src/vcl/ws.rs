//! Store data in a task-centric store to share with the C layers
//!
//! The workspace is a memory allocator with a simple API that allows Varnish to store data that
//! needs only to live for the lifetime of a task (handling a client or backend request for example).
//! At the end of the task, the workspace is wiped, simplifying memory management.
//!
//! Rust handles its own memory, but some data must be shared/returned to the C caller, and the
//! workspace is usually the easiest store available.
//!
//! **Note:** unless you know what you are doing, you should probably just use the automatic type
//! conversion provided by [`crate::vcl::convert`], or store things in
//! [`crate::vcl::vpriv::VPriv`].

use std::any::type_name;
use std::ffi::{c_char, c_void, CStr};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::{align_of, size_of, transmute, MaybeUninit};
use std::num::NonZeroUsize;
use std::ptr;
use std::slice::from_raw_parts_mut;

use memchr::memchr;

#[cfg(lts_60)]
use crate::ffi::WS_Inside;
use crate::ffi::{txt, VCL_BLOB, VCL_STRING};
#[cfg(not(lts_60))]
use crate::ffi::{vrt_blob, WS_Allocated};
use crate::vcl::VclError;
use crate::{ffi, validate_ws};

/// A workspace object
///
/// Used to allocate memory in an efficient manner, data will live there until the end of the
/// transaction and the workspace is wiped, so there's no need to free the objects living in it.
///
/// The workspace is usually a few tens of kilobytes large, don't be greedy. If you need more
/// space, consider storing your data in a `#[shared_per_task]` or `#[shared_per_vcl]` objects.
#[derive(Debug)]
pub struct Workspace<'a> {
    /// Raw pointer to the C struct
    pub raw: *mut ffi::ws,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Workspace<'a> {
    /// Wrap a raw pointer into an object we can use.
    pub(crate) fn from_ptr(raw: *mut ffi::ws) -> Self {
        assert!(!raw.is_null(), "raw pointer was null");
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    /// Allocate a buffer of a given size.
    ///
    /// # Safety
    /// Allocated memory is not initialized.
    pub unsafe fn alloc(&mut self, size: NonZeroUsize) -> *mut c_void {
        #[cfg(not(test))]
        {
            ffi::WS_Alloc(validate_ws(self.raw), size.get() as u32)
        }

        #[cfg(test)]
        {
            // `WS_Alloc` is a private part of `varnishd`, not the Varnish library,
            // so it is only available if the output is a `cdylib`.
            // When testing, VMOD is a lib or a bin,
            // so we have to fake our own allocator.
            let ws = validate_ws(self.raw);
            let align = align_of::<*const c_void>();
            let aligned_sz = ((size.get() + align - 1) / align) * align;
            if ws.e.offset_from(ws.f) < aligned_sz as isize {
                ptr::null_mut()
            } else {
                let p = ws.f.cast::<c_void>();
                ws.f = ws.f.add(aligned_sz);
                p
            }
        }
    }

    /// Check if a pointer is part of the current workspace
    pub fn contains(&self, data: &[u8]) -> bool {
        #[cfg(lts_60)]
        {
            let last = match data.last() {
                None => data.as_ptr(),
                Some(p) => p as *const _,
            };
            unsafe { WS_Inside(self.raw, data.as_ptr().cast(), last.cast()) == 1 }
        }
        #[cfg(not(lts_60))]
        {
            unsafe { WS_Allocated(self.raw, data.as_ptr().cast(), data.len() as isize) == 1 }
        }
    }

    /// Allocate `[u8; size]` array on Workspace.
    /// Returns a reference to uninitialized buffer, or an out of memory error.
    pub fn allocate(&mut self, size: NonZeroUsize) -> Result<&'a mut [MaybeUninit<u8>], VclError> {
        let ptr = unsafe { self.alloc(size) };
        if ptr.is_null() {
            Err(VclError::WsOutOfMemory(size))
        } else {
            Ok(unsafe { from_raw_parts_mut(ptr.cast(), size.get()) })
        }
    }

    /// Allocate `[u8; size]` array on Workspace, and zero it.
    pub fn allocate_zeroed(&mut self, size: NonZeroUsize) -> Result<&'a mut [u8], VclError> {
        let buf = self.allocate(size)?;
        unsafe {
            buf.as_mut_ptr().write_bytes(0, buf.len());
            Ok(slice_assume_init_mut(buf))
        }
    }

    /// Allocate memory on Workspace, and move a value into it.
    /// The value will be dropped in case of out of memory error.
    pub(crate) fn copy_value<T>(&mut self, value: T) -> Result<&'a mut T, VclError> {
        let size = NonZeroUsize::new(size_of::<T>())
            .unwrap_or_else(|| panic!("Type {} has sizeof=0", type_name::<T>()));

        let val = unsafe { self.alloc(size).cast::<T>().as_mut() };
        let val = val.ok_or(VclError::WsOutOfMemory(size))?;
        *val = value;
        Ok(val)
    }

    /// Copy any `AsRef<[u8]>` into the workspace
    fn copy_bytes(&mut self, src: impl AsRef<[u8]>) -> Result<&'a [u8], VclError> {
        // Re-implement unstable `maybe_uninit_write_slice` and `maybe_uninit_slice`
        // See https://github.com/rust-lang/rust/issues/79995
        // See https://github.com/rust-lang/rust/issues/63569
        let src = src.as_ref();
        let Some(len) = NonZeroUsize::new(src.len()) else {
            Err(VclError::CStr(c"Unable to allocate 0 bytes in a Workspace"))?
        };
        let dest = self.allocate(len)?;
        dest.copy_from_slice(maybe_uninit(src));
        Ok(unsafe { slice_assume_init_mut(dest) })
    }

    /*
    /// Copy any `AsRef<[u8]>` into a new [`VCL_BLOB`] stored in the workspace
    pub fn copy_blob(&mut self, value: impl AsRef<[u8]>) -> Result<VCL_BLOB, VclError> {
        let buf = self.copy_bytes(value)?;
        let blob = self.copy_value(vrt_blob {
            blob: ptr::from_ref(buf).cast::<c_void>(),
            len: buf.len(),
            ..Default::default()
        })?;
        Ok(VCL_BLOB(ptr::from_ref(blob)))
    }
    */

    /// Copy any `AsRef<CStr>` into a new [`txt`] stored in the workspace
    pub fn copy_txt(&mut self, value: impl AsRef<CStr>) -> Result<txt, VclError> {
        let dest = self.copy_bytes(value.as_ref().to_bytes_with_nul())?;
        Ok(bytes_with_nul_to_txt(dest))
    }

    /// Copy any `AsRef<CStr>` into a new [`VCL_STRING`] stored in the workspace
    pub fn copy_cstr(&mut self, value: impl AsRef<CStr>) -> Result<VCL_STRING, VclError> {
        Ok(VCL_STRING(self.copy_txt(value)?.b))
    }

    /// Same as [`Workspace::copy_blob`], copying bytes into Workspace, but treats bytes
    /// as a string with an optional NULL character at the end.  A `NULL` is added if it is missing.
    /// Returns an error if `src` contain NULL characters in a non-last position.
    pub fn copy_bytes_with_null(&mut self, src: impl AsRef<[u8]>) -> Result<txt, VclError> {
        let src = src.as_ref();
        match memchr(0, src) {
            Some(pos) if pos + 1 == src.len() => {
                // Safe because there is only one NULL at the end of the buffer.
                self.copy_txt(unsafe { CStr::from_bytes_with_nul_unchecked(src) })
            }
            Some(_) => Err(VclError::CStr(c"NULL byte found in the source string")),
            None => {
                // NUL byte not found, add one at the end
                // Similar to copy_bytes above
                let len = src.len();
                let dest = self.allocate(unsafe { NonZeroUsize::new_unchecked(len + 1) })?;
                dest[..len].copy_from_slice(maybe_uninit(src));
                dest[len].write(b'\0');
                let dest = unsafe { slice_assume_init_mut(dest) };
                Ok(bytes_with_nul_to_txt(dest))
            }
        }
    }

    /// Allocate all the free space in the workspace in a buffer that can be reclaimed or truncated
    /// later.
    ///
    /// Note: don't assume the slice has been zeroed when it is returned to you, see
    /// [`ReservedBuf::release()`] for more information.
    pub fn reserve(&mut self) -> ReservedBuf<'a> {
        let ws = unsafe { validate_ws(self.raw) };

        unsafe {
            let sz = ffi::WS_ReserveAll(ws) as usize;
            let buf = from_raw_parts_mut(ws.f.cast::<u8>(), sz);
            ReservedBuf {
                buf,
                wsp: self.raw,
                b: ws.f.cast::<u8>(),
                len: 0,
            }
        }
    }
}

/// Internal helper to convert a `&[u8]` to a `&[MaybeUninit<u8>]`
fn maybe_uninit(value: &[u8]) -> &[MaybeUninit<u8>] {
    // SAFETY: &[T] and &[MaybeUninit<T>] have the same layout
    // This was copied from MaybeUninit::copy_from_slice, ignoring clippy lints
    unsafe {
        #[allow(clippy::transmute_ptr_to_ptr)]
        transmute(value)
    }
}

/// Internal helper to convert a `&mut [MaybeUninit<u8>]` to a `&[u8]`, assuming all elements are initialized
unsafe fn slice_assume_init_mut(value: &mut [MaybeUninit<u8>]) -> &mut [u8] {
    // SAFETY: Valid elements have just been copied into `this` so it is initialized
    // This was copied from MaybeUninit::slice_assume_init_mut, ignoring clippy lints
    #[allow(clippy::ref_as_ptr)]
    &mut *(value as *mut [MaybeUninit<u8>] as *mut [u8])
}

/// Helper to convert a byte slice with a null terminator to a `txt` struct.
fn bytes_with_nul_to_txt(buf: &[u8]) -> txt {
    txt::from_cstr(unsafe { CStr::from_bytes_with_nul_unchecked(buf) })
}

/// The free region of the workspace. The buffer is fully writable but must be finalized using
/// `release()` to avoid being reclaimed when the struct is dropped.
///
/// Because [`ReservedBuf::release()`] starts counting at the beginning of the slice and because the
/// `Write` traits will actually move that same beginning of the slice, you can
/// `reserve/write/release(0)`:
///
/// ``` ignore
/// // write trait needs to be in scope
/// use std::io::Write;
/// use varnish::vcl::TestWS;
///
/// // init a workspace
/// let mut test_ws = TestWS::new(160);
/// let mut ws = test_ws.ws();
///
/// // first reservation gets the full buffer
/// let mut r = ws.reserve();
/// assert_eq!(r.buf.len(), 160);
///
/// // release AFTER the part we've written
/// r.buf.write(b"0123456789").unwrap();
/// assert_eq!(r.release(0), b"0123456789");
///
/// {
///     // second reservation get 160 - 10 bytes
///     let r2 = ws.reserve();
///     assert_eq!(r2.buf.len(), 150);
///     // the ReservedBuf goes out of scope without a call to .release()
///     // so now data is fully allocated
/// }
///
/// let r3 = ws.reserve();
/// assert_eq!(r3.buf.len(), 150);
/// ```
#[derive(Debug)]
pub struct ReservedBuf<'a> {
    /// The reserved buffer
    pub buf: &'a mut [u8],
    wsp: *mut ffi::ws,
    b: *mut u8,
    len: usize,
}

impl<'a> ReservedBuf<'a> {
    /// Release a [`ReservedBuf`], returning the allocated and now truncated buffer.
    ///
    /// # Safety
    ///
    /// `release` doesn't wipe the unused part of the buffer, so you should not assume that the
    /// slice is pristine when you receive it.
    ///
    /// ``` ignore
    /// use varnish::vcl::TestWS;
    /// let mut test_ws = TestWS::new(160);
    /// let mut ws = test_ws.ws();
    ///
    /// let r = ws.reserve();
    /// r.buf[..9].copy_from_slice(b"IAmNotZero");
    /// r.release(0);
    ///
    /// let r2 = ws.reserve();
    /// assert_eq!(&r2.buf[..9], b"IAmNotZero");
    /// ```
    pub fn release(mut self, sz: usize) -> &'a mut [u8] {
        unsafe {
            self.len = self.buf.as_ptr().add(sz).offset_from(self.b) as usize;
            from_raw_parts_mut(self.b, self.len)
        }
    }
}

impl<'a> Drop for ReservedBuf<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::WS_Release(validate_ws(self.wsp), self.len as u32);
        }
    }
}

/// A struct holding both a native ws struct and the space it points to.
///
/// As the name implies, this struct mainly exist to facilitate testing and should probably not be
/// used elsewhere.
#[derive(Debug)]
pub struct TestWS {
    c_ws: ffi::ws,
    #[allow(dead_code)]
    space: Vec<c_char>,
}

impl TestWS {
    /// Instantiate a `C` ws struct and the required space of size `sz`.
    pub fn new(sz: usize) -> Self {
        let al = align_of::<*const c_void>();
        let aligned_sz = (sz / al) * al;
        let mut v: Vec<c_char> = vec![0; sz];
        let s = v.as_mut_ptr();
        Self {
            c_ws: ffi::ws {
                magic: ffi::WS_MAGIC,
                id: ['t' as c_char, 's' as c_char, 't' as c_char, '\0' as c_char],
                s,
                f: s,
                r: ptr::null_mut(),
                e: unsafe { s.add(aligned_sz) },
            },
            space: v,
        }
    }

    /// Return a pointer to the underlying C ws struct. As usual, the caller needs to ensure that
    /// self doesn't outlive the returned pointer.
    pub fn as_ptr(&mut self) -> *mut ffi::ws {
        ptr::from_mut::<ffi::ws>(&mut self.c_ws)
    }

    /// build a `Workspace`
    pub fn workspace(&mut self) -> Workspace {
        Workspace::from_ptr(self.as_ptr())
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZero;

    use super::*;

    #[test]
    fn ws_test() {
        let mut test_ws = TestWS::new(160);
        let mut ws = test_ws.workspace();
        for _ in 0..10 {
            unsafe {
                assert!(!ws.alloc(NonZero::new(16).unwrap()).is_null());
            }
        }
        unsafe {
            assert!(ws.alloc(NonZero::new(1).unwrap()).is_null());
        }
    }
}
