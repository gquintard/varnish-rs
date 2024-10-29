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
use std::ffi::{c_char, c_void, CStr};
use std::marker::PhantomData;
use std::ptr;
use std::slice::from_raw_parts_mut;
use std::str::from_utf8;

use crate::ffi::WS_Allocated;
use crate::{ffi, validate_ws};

/// A workspace object
///
/// Used to allocate memory in an efficient manner, data will live there until the end of the
/// transaction and the workspace is wiped, so there's no need to free the objects living in it.
///
/// The workspace is usually a few tens of kilobytes large, don't be greedy. If you need more
/// space, consider storing your data in a [`VPriv`](crate::vcl::vpriv::VPriv).
#[derive(Debug)]
pub struct Workspace<'a> {
    /// Raw pointer to the C struct
    pub raw: *mut ffi::ws,
    phantom_a: PhantomData<&'a u8>,
}

impl<'a> Workspace<'a> {
    /// Wrap a raw pointer into an object we can use.
    pub fn new(raw: *mut ffi::ws) -> Self {
        assert!(!raw.is_null(), "raw pointer was null");
        Self {
            raw,
            phantom_a: PhantomData,
        }
    }

    /// Allocate a `[u8; sz]` and return a reference to it.
    #[cfg(not(test))]
    pub fn alloc(&mut self, sz: usize) -> Result<&'a mut [u8], String> {
        let ws = unsafe { validate_ws(self.raw) };
        let p = unsafe { ffi::WS_Alloc(ws, sz as u32).cast::<u8>() };
        if p.is_null() {
            Err(format!("workspace allocation ({sz} bytes) failed"))
        } else {
            unsafe { Ok(from_raw_parts_mut(p, sz)) }
        }
    }

    #[cfg(test)]
    pub fn alloc(&mut self, sz: usize) -> Result<&'a mut [u8], String> {
        let ws = unsafe { validate_ws(self.raw) };
        let al = align_of::<*const c_void>();
        let aligned_sz = ((sz + al - 1) / al) * al;

        unsafe {
            if ws.e.offset_from(ws.f) < aligned_sz as isize {
                Err(format!(
                    "not enough room for {aligned_sz} (rounded up from {sz}). f: {:?}, e: {:?}",
                    ws.f, ws.e
                ))
            } else {
                let buf = from_raw_parts_mut(ws.f.cast::<u8>(), aligned_sz);
                ws.f = ws.f.add(aligned_sz);
                Ok(buf)
            }
        }
    }

    /// Check if a byte slice is allocated in the workspace
    pub fn is_slice_allocated(&self, slice: &[u8]) -> bool {
        unsafe {
            WS_Allocated(
                self.raw,
                slice.as_ptr().cast::<c_void>(),
                slice.len() as isize + 1,
            ) == 1
        }
    }

    /// Copy any struct implementing `AsRef<[u8]>` into the workspace
    pub fn copy_bytes<T: AsRef<[u8]>>(&mut self, src: &T) -> Result<&'a [u8], String> {
        let buf = src.as_ref();
        let l = buf.len();

        let dest = self.alloc(l)?;
        dest.copy_from_slice(buf);
        Ok(dest)
    }

    /// Same as [`Workspace::copy_bytes`] but adds NULL character at the end to help converts buffers into
    /// `VCL_STRING`s. Returns an error if `src` contain NULL characters.
    pub fn copy_bytes_with_null<T: AsRef<[u8]>>(&mut self, src: &T) -> Result<&'a CStr, String> {
        let buf = src.as_ref();
        if buf.contains(&0) {
            return Err("source buffer contains NULL character".into());
        }
        let l = buf.len();
        let dest = self.alloc(l + 1)?;
        dest[..l].copy_from_slice(buf);
        dest[l] = b'\0';
        // Safe because there are no NULLs in the source, and we just added a NULL
        Ok(unsafe { CStr::from_bytes_with_nul_unchecked(dest) })
    }

    /// Copy any "`str`-like" struct into the workspace
    pub fn copy_str<T: AsRef<str>>(&mut self, src: &T) -> Result<&'a str, String> {
        let s: &str = src.as_ref();
        let buf = s.as_bytes();
        let l = buf.len();

        let dest = self.alloc(l)?;
        dest.copy_from_slice(buf);
        Ok(from_utf8(dest).unwrap())
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
        &mut self.c_ws as *mut ffi::ws
    }

    /// build a `Workspace`
    pub fn workspace(&mut self) -> Workspace {
        Workspace::new(self.as_ptr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_test() {
        let mut test_ws = TestWS::new(160);
        let mut ws = test_ws.workspace();
        for _ in 0..10 {
            let r = ws.alloc(16);
            assert!(r.is_ok());
            let buf = r.unwrap();
            assert_eq!(buf.len(), 16);
        }
        assert!(ws.alloc(1).is_err());
    }
}
