use crate::ffi;
use crate::ffi::{vrt_blob, VCL_BLOB, VCL_STRING};
use std::io::Write;
use std::ops::Add;
use std::slice::from_raw_parts_mut;
use std::{io, mem};

/// The free region of the workspace that can be written using the [Write] methods.
/// The buffer must be finalized using `finish()` to avoid being reclaimed when dropped.
#[derive(Debug)]
pub struct WsBuffer<'a, T> {
    /// The workspace pointer, used to release the workspace
    ws: &'a mut ffi::ws,
    /// The start of the writable buffer
    start: *mut u8,
    /// The reserved buffer will move its start as we write to it
    buffer: &'a mut [u8],
    /// The type of the buffer
    _marker: std::marker::PhantomData<T>,
}

pub type WsStrBuffer<'a> = WsBuffer<'a, VCL_STRING>;
pub type WsBlobBuffer<'a> = WsBuffer<'a, VCL_BLOB>;

impl<T> WsBuffer<'_, T> {
    /// Get the written data as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let len = self.buffer.as_ptr().offset_from(self.start) as usize;
            std::slice::from_raw_parts(self.start, len)
        }
    }

    /// Get the length of the written data
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// Check if anything has been written to the buffer
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the remaining capacity of the buffer
    pub fn remaining(&self) -> usize {
        self.buffer.len()
    }

    /// Release the workspace, reclaiming the memory except for the written data.
    ///
    /// Safety:
    ///     This must be the last call before dropping. May be called multiple times.
    unsafe fn release(&mut self, len: u32) {
        if !mem::replace(&mut self.start, std::ptr::null_mut()).is_null() {
            self.ws.release(len);
        }
    }
}

impl<'a> WsStrBuffer<'a> {
    /// Create a new string buffer from the workspace
    pub(crate) unsafe fn new(ws: &'a mut ffi::ws) -> Self {
        let available_space = ws.reserve_all() as usize;

        // FIXME: use Result instead
        assert!(available_space > 0, "Workspace too small to hold a string");

        let start = ws.f.cast::<u8>();
        WsBuffer {
            ws,
            start,
            // Reserve extra space for the NUL terminator (in case of StrStore)
            buffer: from_raw_parts_mut(start, available_space - 1),
            _marker: std::marker::PhantomData,
        }
    }

    /// Finish writing to the [`WsBuffer`], returning the allocated [`VCL_STRING`].
    pub fn finish(mut self) -> VCL_STRING {
        unsafe {
            let start = self.ws.f.cast::<u8>();
            let next_available_byte = self.buffer.as_mut_ptr();
            // SAFETY:
            // Since we reserved one extra byte for the NUL terminator,
            // we can force write the NUL terminator even past the end of the slice.
            next_available_byte.write(b'\0');
            // Reserve written data including the NUL terminator, and release the rest
            self.release(next_available_byte.offset_from(start).add(1) as u32);
            VCL_STRING(start.cast())
        }
    }
}

const VRT_BLOB_SIZE: usize = size_of::<vrt_blob>();

impl<'a> WsBlobBuffer<'a> {
    /// Create a new data blob buffer from the workspace
    pub(crate) unsafe fn new(ws: &'a mut ffi::ws) -> Self {
        let sz = ws.reserve_all() as usize;

        let raw_start = ws.f;
        let offset = raw_start.align_offset(align_of::<vrt_blob>());

        // FIXME: this is a valid error, should return Result instead
        assert!(
            sz >= VRT_BLOB_SIZE + offset,
            "Workspace too small to hold a blob"
        );

        // Reserve space for the vrt_blob struct that will go in front of the data
        let start = raw_start.add(offset).add(VRT_BLOB_SIZE).cast::<u8>();
        let len = sz - offset - VRT_BLOB_SIZE;
        // Assuming u8 is always aligned
        debug_assert!(start.is_aligned(), "WS buffer is not aligned");

        WsBuffer {
            ws,
            start,
            buffer: from_raw_parts_mut(start, len),
            _marker: std::marker::PhantomData,
        }
    }

    /// Finish writing to the [`WsBuffer`], returning the allocated [`VCL_BLOB`].
    pub fn finish(mut self) -> VCL_BLOB {
        unsafe {
            let raw_start = self.ws.f.cast::<u8>();
            let offset = raw_start.align_offset(align_of::<vrt_blob>());
            #[expect(clippy::cast_ptr_alignment)]
            let info = raw_start.add(offset).cast::<vrt_blob>();
            let data = self.as_bytes();
            // Save the data length in the vrt_blob struct at the begging of the buffer
            info.cast::<vrt_blob>().write(vrt_blob {
                blob: data.as_ptr().cast(),
                len: data.len(),
                ..Default::default()
            });
            let next_available_byte = self.buffer.as_mut_ptr();
            self.release(next_available_byte.offset_from(raw_start) as u32);

            VCL_BLOB(info)
        }
    }
}

impl<T> Write for WsBuffer<'_, T> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buffer.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<T> Drop for WsBuffer<'_, T> {
    /// Ignore all the write commands, reclaiming the workspace memory
    fn drop(&mut self) {
        unsafe { self.release(0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcl::TestWS;
    use std::ffi::CStr;

    fn get_cstr(s: &VCL_STRING) -> &CStr {
        unsafe { CStr::from_ptr(s.0) }
    }

    #[test]
    #[ignore] // TODO: implement ws::reserve_all and ws::release
    fn str_buffer() {
        // init a workspace
        let mut test_ws = TestWS::new(160);
        let mut ws = test_ws.workspace();

        // first buffer call gets all available space
        let mut buf = ws.get_str_buffer();
        assert_eq!(buf.remaining(), 160);

        // Validating writing
        buf.write_all(b"0123456789").unwrap();
        let res = buf.finish();
        assert_eq!(get_cstr(&res), c"0123456789");

        // second reservation get 160 - 10 bytes
        let mut buf = ws.get_str_buffer();
        assert_eq!(buf.remaining(), 150);
        write!(buf, "this data is ignored").unwrap();
        // the ReservedBuf goes out of scope without a call to .finish()
        // so now data is fully allocated

        let buf = ws.get_str_buffer();
        assert_eq!(buf.remaining(), 150);
    }

    #[test]
    #[ignore] // TODO: implement ws::reserve_all and ws::release
    fn blob_buffer() {
        // init a workspace
        let mut test_ws = TestWS::new(160);
        let mut ws = test_ws.workspace();

        // first buffer call gets all available space
        let mut buf = ws.get_blob_buffer();
        assert_eq!(buf.remaining(), 160 - VRT_BLOB_SIZE);

        // Validating writing
        buf.write_all(b"0123456789").unwrap();
        let res = buf.finish();
        let vrt_blob { blob, len, .. } = unsafe { *(res.0) };
        let data = unsafe { std::slice::from_raw_parts(blob.cast::<u8>(), len) };
        assert_eq!(data, b"0123456789");

        // second reservation get 160 - 10 bytes
        let mut buf = ws.get_str_buffer();
        assert_eq!(buf.remaining(), 150 - VRT_BLOB_SIZE);
        write!(buf, "this data is ignored").unwrap();
        // the ReservedBuf goes out of scope without a call to .finish()
        // so now data is fully allocated

        let buf = ws.get_str_buffer();
        assert_eq!(buf.remaining(), 150 - VRT_BLOB_SIZE);
    }
}
