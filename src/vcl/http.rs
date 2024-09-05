//! Headers and top line of an HTTP object
//!
//! Depending on the VCL subroutine, the `Ctx` will give access to various [`HTTP`] object which
//! expose the request line (`req`, `req_top` and `bereq`), response line (`resp`, `beresp`) and
//! headers of the objects Varnish is manipulating.
//!
//! `HTTP` implements `IntoIterator` that will expose the headers only (not the `method`, `status`,
//! etc.)
//!
//! **Note:** at this stage, headers are assumed to be utf8, and you will get a panic if it's not
//! the case. Future work needs to sanitize the headers to make this safer to use. It is tracked in
//! this [issue](https://github.com/gquintard/varnish-rs/issues/4).

#![allow(clippy::not_unsafe_ptr_arg_deref)]
use std::ffi::c_uint;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::str::from_utf8;

use crate::ffi;
use crate::vcl::ws::WS;
use crate::vcl::Result;

// C constants pop up as u32, but header indexing uses u16, redefine
// some stuff to avoid casting all the time
const HDR_FIRST: u16 = ffi::HTTP_HDR_FIRST as u16;
const HDR_METHOD: u16 = ffi::HTTP_HDR_METHOD as u16;
const HDR_PROTO: u16 = ffi::HTTP_HDR_PROTO as u16;
const HDR_REASON: u16 = ffi::HTTP_HDR_REASON as u16;
const HDR_STATUS: u16 = ffi::HTTP_HDR_STATUS as u16;
const HDR_UNSET: u16 = ffi::HTTP_HDR_UNSET as u16;
const HDR_URL: u16 = ffi::HTTP_HDR_URL as u16;

/// HTTP headers of an object
pub struct HTTP<'a> {
    pub raw: &'a mut ffi::http,
}

impl<'a> HTTP<'a> {
    /// Wrap a raw pointer into an object we can use.
    pub fn new(p: *mut ffi::http) -> Option<Self> {
        Some(HTTP {
            raw: unsafe { p.as_mut()? },
        })
    }

    fn change_header(&mut self, idx: u16, value: &str) -> Result<()> {
        assert!(idx < self.raw.nhd);

        /* XXX: aliasing warning, it's the same pointer as the one in Ctx */
        let mut ws = WS::new(self.raw.ws);
        unsafe {
            let hdr_buf = ws.copy_bytes_with_null(&value)?;
            let hd = self.raw.hd.offset(idx as isize);
            (*hd).b = hdr_buf.as_ptr();
            // .e points to the NULL byte at the end of the string
            (*hd).e = hdr_buf.as_ptr().add(hdr_buf.count_bytes());
            let hdf = self.raw.hdf.offset(idx as isize);
            *hdf = 0;
        }
        Ok(())
    }

    /// Append a new header using `name` and `value`. This can fail if we run out of internal slots
    /// to store the new header
    pub fn set_header(&mut self, name: &str, value: &str) -> Result<()> {
        assert!(self.raw.nhd <= self.raw.shd);
        if self.raw.nhd == self.raw.shd {
            return Err("no more header slot".into());
        }

        let idx = self.raw.nhd;
        self.raw.nhd += 1;
        let res = self.change_header(idx, &format!("{name}: {value}"));
        if res.is_ok() {
            unsafe {
                ffi::VSLbt(
                    self.raw.vsl,
                    self.raw.logtag as c_uint + HDR_FIRST as c_uint,
                    *self.raw.hd.add(idx as usize),
                );
            }
        } else {
            self.raw.nhd -= 1;
        }
        res
    }

    pub fn unset_header(&mut self, name: &str) {
        let hdrs = unsafe {
            &from_raw_parts_mut(self.raw.hd, self.raw.nhd as usize)[(HDR_FIRST as usize)..]
        };

        let mut idx_empty = 0;
        for (idx, hd) in hdrs.iter().enumerate() {
            let (n, _) = header_from_hd(hd).unwrap();
            if name.eq_ignore_ascii_case(n) {
                unsafe {
                    ffi::VSLbt(
                        self.raw.vsl,
                        self.raw.logtag + HDR_UNSET as u32 - HDR_METHOD as u32,
                        *self.raw.hd.add(HDR_FIRST as usize + idx),
                    );
                }
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
                    let buf = from_raw_parts(b.cast::<u8>(), e.offset_from(b) as usize);
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

    /// Set prototype
    pub fn set_proto(&mut self, value: &str) -> Result<()> {
        self.raw.protover = match value {
            "HTTP/0.9" => 9,
            "HTTP/1.0" => 10,
            "HTTP/1.1" => 11,
            "HTTP/2.0" => 20,
            _ => 0,
        };
        self.change_header(HDR_PROTO, value)
    }

    /// Response status, `None` for a request
    pub fn status(&self) -> Option<&str> {
        self.field(HDR_STATUS)
    }

    /// Set the response status, it will also set the reason
    pub fn set_status(&mut self, status: u16) {
        unsafe { ffi::http_SetStatus(self.raw, status, std::ptr::null()) }
    }

    /// Response reason, `None` for a request
    pub fn reason(&self) -> Option<&str> {
        self.field(HDR_REASON)
    }

    /// Set reason
    pub fn set_reason(&mut self, value: &str) -> Result<()> {
        self.change_header(HDR_REASON, value)
    }

    /// Returns the value of a header based on its name
    ///
    /// The header names are compared in a case-insensitive manner
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
            let nhd = self.http.raw.nhd;
            if self.cursor >= nhd as isize {
                return None;
            }
            let hd = unsafe { self.http.raw.hd.offset(self.cursor) };
            self.cursor += 1;
            if let Some(hdr) = header_from_hd(hd) {
                return Some(hdr);
            }
        }
    }
}

fn header_from_hd<'a>(txt: *const ffi::txt) -> Option<(&'a str, &'a str)> {
    let name;
    let value;

    unsafe {
        let b = (*txt).b;
        if b.is_null() {
            return None;
        }
        let e = (*txt).e;
        let buf = from_raw_parts(b.cast::<u8>(), e.offset_from(b) as usize);
        // We expect varnishd to always given us a string with a ':' in it
        // If it's not the case, blow up as it might be a sign of a bigger problem.
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
