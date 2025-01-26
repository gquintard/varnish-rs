use std::ops::{Deref, DerefMut};
use std::ffi::CString;
use std::mem::size_of;
use varnish_sys::ffi::{vsc_seg, VRT_VSC_Alloc, VRT_VSC_Destroy};

pub unsafe trait VscMetric {
    fn get_metadata() -> &'static str;
}

pub struct Vsc<T: VscMetric> {
    metric: *mut T,
    seg: *mut vsc_seg,
    name: CString,
}

impl<T: VscMetric> Vsc<T> {
    pub fn new(module_name: &str, module_prefix: &str) -> Self {
        let mut seg = std::ptr::null_mut();
        let name = CString::new(module_name).unwrap();
        let format = CString::new(module_prefix).unwrap();
        
        let metadata_json = T::get_metadata();

        let metric = unsafe {
            VRT_VSC_Alloc(
                std::ptr::null_mut(),
                &mut seg,
                name.as_ptr(),
                size_of::<T>(),
                metadata_json.as_ptr(),
                metadata_json.len(),
                format.as_ptr(),
                std::ptr::null_mut()
            ).cast::<T>()
        };

        Self { 
            metric,
            seg,
            name,
        }
    }
}

impl<T: VscMetric> Drop for Vsc<T> {
    fn drop(&mut self) {
        unsafe {
            VRT_VSC_Destroy(self.name.as_ptr(), self.seg);
        }
    }
}

impl<T: VscMetric> Deref for Vsc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.metric }
    }
}

impl<T: VscMetric> DerefMut for Vsc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.metric }
    }
}