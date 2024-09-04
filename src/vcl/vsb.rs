//! VSB, growable buffer

use crate::ffi;

pub struct Vsb<'a> {
    /// Raw pointer to the C struct
    pub raw: &'a mut ffi::vsb,
}

impl<'a> Vsb<'a> {
    /// Create a `Vsb` from a C pointer
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn new(raw: *mut ffi::vsb) -> Self {
        let raw = unsafe { raw.as_mut().unwrap() };
        assert_eq!(raw.magic, ffi::VSB_MAGIC);
        Vsb { raw }
    }

    /// Push a buffer into the `Vsb`
    pub fn cat<T: AsRef<[u8]>>(&mut self, src: &T) -> Result<(), ()> {
        let buf = src.as_ref().as_ptr().cast::<std::ffi::c_void>();
        let l = src.as_ref().len();

        match unsafe { ffi::VSB_bcat(self.raw, buf, l as isize) } {
            0 => Ok(()),
            _ => Err(()),
        }
    }
}
