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
use std::ffi::c_void;
use std::ptr;
use std::slice::from_raw_parts_mut;

/// A workspace object
///
/// Used to allocate memory in an efficient manner, data will live there until the end of the
/// transaction and the workspace is wiped, so there's no need to free the objects living in it.
///
/// The workspace is usually a few tens of kilobytes large, don't be greedy. If you need more
/// space, consider storing your data in a [`VPriv`](crate::vcl::vpriv::VPriv).
pub struct WS<'a> {
    pub raw: *mut varnish_sys::ws,
    phantom_a: std::marker::PhantomData<&'a u8>,
}

impl<'a> WS<'a> {
    /// Wrap a raw pointer into an object we can use.
    pub fn new(raw: *mut varnish_sys::ws) -> Self {
        if raw.is_null() {
            panic!("raw pointer was null");
        }
        WS {
            raw,
            phantom_a: std::marker::PhantomData,
        }
    }

    /// Allocate a `[u8; sz]` and return a reference to it.
    #[cfg(not(test))]
    pub fn alloc(&mut self, sz: usize) -> Result<&'a mut [u8], String> {
        let p = unsafe { varnish_sys::WS_Alloc(self.raw, sz as u32) as *mut u8 };
        if p.is_null() {
            Err(format!("workspace allocation ({} bytes) failed", sz))
        } else {
            unsafe { Ok(from_raw_parts_mut(p, sz)) }
        }
    }
    #[cfg(test)]
    pub fn alloc(&mut self, sz: usize) -> Result<&'a mut [u8], String> {
        let mut wsp = unsafe { self.raw.as_mut().unwrap() };
        assert_eq!(wsp.magic, varnish_sys::WS_MAGIC);

        let al = std::mem::align_of::<*const c_void>();
        let aligned_sz = ((sz + al - 1) / al) * al;

        unsafe {
            if wsp.e.offset_from(wsp.f) < aligned_sz as isize {
                Err(format!(
                    "not enough room for {} (rounded up from {}). f: {:?}, e: {:?}",
                    aligned_sz, sz, wsp.f, wsp.e
                ))
            } else {
                let buf = from_raw_parts_mut(wsp.f as *mut u8, aligned_sz);
                wsp.f = wsp.f.add(aligned_sz);
                Ok(buf)
            }
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

    /// Copy any "`str`-like" struct into the workspace
    pub fn copy_str<T: AsRef<str>>(&mut self, src: &T) -> Result<&'a str, String> {
        let s: &str = src.as_ref();
        let buf = s.as_bytes();
        let l = buf.len();

        let dest = self.alloc(l)?;
        dest.copy_from_slice(buf);
        Ok(std::str::from_utf8(dest).unwrap())
    }
}

/// A struct holding both a native ws struct, as well as the space it points to.
///
/// As the name implies, this struct mainly exist to facilitate testing and should probably not be
/// used elsewhere.
pub struct TestWS {
    c_ws: varnish_sys::ws,
    #[allow(dead_code)]
    space: Vec<i8>,
}

impl TestWS {
    /// Instantiate a `C` ws struct and the required space of size `sz`.
    pub fn new(sz: usize) -> Self {
        let al = std::mem::align_of::<*const c_void>();
        let aligned_sz = (sz / al) * al;

        let mut v = Vec::new();
        v.resize(sz, 0);

        let s = v.as_mut_ptr();
        TestWS {
            c_ws: varnish_sys::ws {
                magic: varnish_sys::WS_MAGIC,
                id: ['t' as i8, 's' as i8, 't' as i8, '\0' as i8],
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
    pub fn as_ptr(&mut self) -> *mut varnish_sys::ws {
        &mut self.c_ws as *mut varnish_sys::ws
    }

    /// build a `WS`
    pub fn ws(&mut self) -> WS {
        WS::new(self.as_ptr())
    }
}

#[test]
fn ws_test() {
    let mut test_ws = TestWS::new(160);
    let mut ws = test_ws.ws();
    for _ in 0..10 {
        let r = ws.alloc(16);
        assert!(r.is_ok());
        let buf = r.unwrap();
        assert_eq!(buf.len(), 16);
    }
    assert!(ws.alloc(1).is_err());
}
