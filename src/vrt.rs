//! The Varnish RunTime wrapper
//!
//! This module provides access to the public API use by VCL and vmods.

#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::str::from_utf8;

// C constants pop up as u32, but header indexing uses u16, redefine
// some stuff to avoid casting all the time
const HDR_FIRST: u16 = varnish_sys::HTTP_HDR_FIRST as u16;
const HDR_METHOD: u16 = varnish_sys::HTTP_HDR_METHOD as u16;
const HDR_PROTO: u16 = varnish_sys::HTTP_HDR_PROTO as u16;
const HDR_REASON: u16 = varnish_sys::HTTP_HDR_REASON as u16;
const HDR_STATUS: u16 = varnish_sys::HTTP_HDR_STATUS as u16;
const HDR_URL: u16 = varnish_sys::HTTP_HDR_URL as u16;

/// VCL context
///
/// A mutable reference to this structure is always passed to vmod functions and provides access to
/// the available HTTP objects, as well as the workspace.
///
/// This struct is a pure Rust structure, mirroring some of the C fields, so you should always use
/// the provided methods to interact with them. If they are not enough, the `raw` field is actually
/// the C original pointer that can be used to directly, and unsafely, act on the structure.
///
/// Which `http_*` are present will depend on which VCL sub routine the function is called from.
pub struct Ctx<'a> {
    pub raw: *const varnish_sys::vrt_ctx,
    pub http_req: Option<HTTP<'a>>,
    pub http_req_top: Option<HTTP<'a>>,
    pub http_resp: Option<HTTP<'a>>,
    pub http_bereq: Option<HTTP<'a>>,
    pub http_beresp: Option<HTTP<'a>>,
    pub ws: WS<'a>,
}

impl<'a> Ctx<'a> {
    /// Wrap a raw pointer into an object we can use.
    ///
    /// The pointer must be non-null, and the magic must match
    pub fn new(raw: *mut varnish_sys::vrt_ctx) -> Self {
        let p = unsafe { raw.as_ref().unwrap() };
        assert_eq!(p.magic, varnish_sys::VRT_CTX_MAGIC);
        Ctx {
            raw,
            http_req: HTTP::new(p.http_req),
            http_req_top: HTTP::new(p.http_req_top),
            http_resp: HTTP::new(p.http_resp),
            http_bereq: HTTP::new(p.http_bereq),
            http_beresp: HTTP::new(p.http_beresp),
            ws: WS::new(p.ws),
        }
    }
}

/// HTTP headers of an object
pub struct HTTP<'a> {
    pub raw: &'a mut varnish_sys::http,
}

impl<'a> HTTP<'a> {
    /// Wrap a raw pointer into an object we can use.
    pub fn new(p: *mut varnish_sys::http) -> Option<Self> {
        if p.is_null() {
            None
        } else {
            Some(HTTP {
                raw: unsafe { p.as_mut().unwrap() },
            })
        }
    }

    fn change_header(&mut self, idx: u16, name: &str, value: &str) -> Result<(), String> {
        assert!(idx < self.raw.nhd);

        /* XXX: aliasing warning, it's the same pointer as the one in Ctx */
        let mut ws = WS::new(self.raw.ws);
        let hdr_buf = ws.copy_bytes(&format!("{}: {}\0", name, value))?;
        unsafe {
            let mut hd = self.raw.hd.offset(idx as isize);
            (*hd).b = hdr_buf.as_ptr() as *const i8;
            /* -1 accounts for the null character */
            (*hd).e = hdr_buf.as_ptr().add(hdr_buf.len() - 1) as *const i8;
            let hdf = self.raw.hdf.offset(idx as isize);
            *hdf = 0;
        }
        Ok(())
    }

    /// Append a new header using `name` and `value`. This can fail if we run out of internal slot
    /// to store the new header
    pub fn set_header(&mut self, name: &str, value: &str) -> Result<(), String> {
        assert!(self.raw.nhd <= self.raw.shd);
        if self.raw.nhd == self.raw.shd {
            return Err("no more header slot".to_string());
        }

        let idx = self.raw.nhd;
        self.raw.nhd += 1;
        self.change_header(idx, name, value)
    }

    pub fn unset_header(&mut self, name: &str) {
        let hdrs = unsafe {
            &from_raw_parts_mut(self.raw.hd, self.raw.nhd as usize)[(HDR_FIRST as usize)..]
        };

        let mut idx_empty = 0;
        for (idx, hd) in hdrs.iter().enumerate() {
            let (n, _) = header_from_hd(hd).unwrap();
            if name.eq_ignore_ascii_case(n) {
                continue;
            }
            if idx != idx_empty {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        self.raw.hd.add(HDR_FIRST as usize + idx),
                        self.raw.hd.add(HDR_FIRST as usize + idx_empty),
                        1,
                    );
                    std::ptr::copy_nonoverlapping(
                        self.raw.hdf.add(HDR_FIRST as usize + idx),
                        self.raw.hdf.add(HDR_FIRST as usize + idx_empty),
                        1,
                    );
                }
            }
            idx_empty += 1;
        }
        self.raw.nhd = HDR_FIRST + idx_empty as u16;
    }

    /// Return header at a specific position
    fn field(&self, idx: u16) -> Option<&str> {
        unsafe {
            if idx >= self.raw.nhd {
                None
            } else {
                let b = (*self.raw.hd.offset(idx as isize)).b;
                if b.is_null() {
                    None
                } else {
                    let e = (*self.raw.hd.offset(idx as isize)).e;
                    let buf = from_raw_parts(b as *const u8, e.offset_from(b) as usize);
                    Some(from_utf8(buf).unwrap())
                }
            }
        }
    }

    /// Method of an HTTP request, `None` for a response
    pub fn method(&self) -> Option<&str> {
        self.field(HDR_METHOD)
    }

    /// URL of an HTTP request, `None` for a response
    pub fn url(&self) -> Option<&str> {
        self.field(HDR_URL)
    }

    /// Protocol of an object
    ///
    /// It should exist for both requests and responses, but the `Option` is maintained for
    /// consistency.
    pub fn proto(&self) -> Option<&str> {
        self.field(HDR_PROTO)
    }

    /// Response status, `None` for a request
    pub fn status(&self) -> Option<&str> {
        self.field(HDR_STATUS)
    }

    /// Response reason, `None` for a request
    pub fn reason(&self) -> Option<&str> {
        self.field(HDR_REASON)
    }

    /// Returns the value of a header based on its name
    ///
    /// The header names are compare in a case-insensitive manner
    pub fn header(&self, name: &str) -> Option<&str> {
        self.into_iter()
            .find(|hdr| name.eq_ignore_ascii_case(hdr.0))
            .map(|hdr| hdr.1)
    }
}

impl<'a> IntoIterator for &'a HTTP<'a> {
    type Item = (&'a str, &'a str);
    type IntoIter = HTTPIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        HTTPIter {
            http: self,
            cursor: HDR_FIRST as isize,
        }
    }
}

impl<'a> IntoIterator for &'a mut HTTP<'a> {
    type Item = (&'a str, &'a str);
    type IntoIter = HTTPIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        HTTPIter {
            http: self,
            cursor: HDR_FIRST as isize,
        }
    }
}

pub struct HTTPIter<'a> {
    http: &'a HTTP<'a>,
    cursor: isize,
}

impl<'a> Iterator for HTTPIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let nhd = (*self.http.raw).nhd;
            if self.cursor >= nhd as isize {
                return None;
            } else {
                let hd = unsafe { self.http.raw.hd.offset(self.cursor) };
                self.cursor += 1;
                match header_from_hd(hd) {
                    None => continue,
                    Some(hdr) => return Some(hdr),
                }
            }
        }
    }
}

fn header_from_hd<'a>(txt: *const varnish_sys::txt) -> Option<(&'a str, &'a str)> {
    let name;
    let value;

    unsafe {
        let b = (*txt).b;
        if b.is_null() {
            return None;
        }
        let e = (*txt).e;
        let buf = from_raw_parts(b as *const u8, e.offset_from(b) as usize);

        let colon = buf.iter().position(|x| *x == b':').unwrap();

        name = from_utf8(&buf[..colon]).unwrap();

        if colon == buf.len() - 1 {
            value = "";
        } else {
            let start = &buf[colon + 1..]
                .iter()
                .position(|x| !char::is_whitespace(*x as char))
                .unwrap_or(0);
            value = from_utf8(&buf[(colon + start + 1)..]).unwrap();
        }
    }
    Some((name, value))
}

/// A workspace object
///
/// Used to allocate memory in an efficient manner, data will live there until the end of the
/// transaction and the workspace is wiped, so there's no need to free the objects living in it.
///
/// The workspace is usually a few tens of kilobytes large, don't be greedy. If you need more
/// space, consider storing your data in a [`VPriv`](crate::vmod::vpriv::VPriv).
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

    /// Allocate a `[u8]` and return a reference to it
    pub fn alloc(&mut self, size: usize) -> Result<&'a mut [u8], String> {
        let p = unsafe { varnish_sys::WS_Alloc(self.raw, size as u32) as *mut u8 };
        if p.is_null() {
            Err(format!("workspace allocation ({} bytes) failed", size))
        } else {
            unsafe { Ok(from_raw_parts_mut(p, size)) }
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
