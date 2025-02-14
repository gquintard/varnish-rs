use crate::ffi;
use crate::ffi::VCL_STRING;
use std::io;
use std::io::Write;
use std::ops::Add;
use std::slice::from_raw_parts_mut;

/// The free region of the workspace that can be written using the [Write] methods.
/// The buffer must be finalized using `finish()` to avoid being reclaimed when dropped.
#[derive(Debug)]
pub struct WsStrBuffer<'a> {
    /// Whether the workspace has been released
    is_released: bool,
    /// The workspace pointer, used to release the workspace
    ws: &'a mut ffi::ws,
    /// The reserved buffer will move its start as we write to it
    buffer: &'a mut [u8],
}

impl<'a> WsStrBuffer<'a> {
    /// Create a new string buffer from the workspace
    pub(crate) unsafe fn new(ws: &'a mut ffi::ws) -> Self {
        let sz = ws.reserve_all();
        // Workspace API implies that the size is a u32
        debug_assert!(u32::try_from(sz).is_ok());
        let data = ws.f.cast::<u8>();
        // Reserve extra space for the NUL terminator
        let len = sz - 1;
        WsStrBuffer {
            is_released: false,
            ws,
            buffer: from_raw_parts_mut(data, len),
        }
    }

    /// Get the written data as a byte slice
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let start = self.ws.f.cast::<u8>();
            let len = self.buffer.as_ptr().offset_from(start) as usize;
            std::slice::from_raw_parts(start, len)
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

    /// Finish writing to the [`WsStrBuffer`], returning the allocated [`VCL_STRING`].
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

    /// Release the workspace, reclaiming the memory except for the written data.
    ///
    /// Safety:
    ///     This must be the last call before dropping. May be called multiple times.
    unsafe fn release(&mut self, len: u32) {
        if !self.is_released {
            self.is_released = true;
            self.ws.release(len);
        }
    }
}

impl Write for WsStrBuffer<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buffer.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for WsStrBuffer<'_> {
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
    fn multi_write() {
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
}
