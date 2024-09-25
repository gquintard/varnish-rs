use std::any::type_name;
use std::ffi::{c_char, c_void, CString};
use std::marker::PhantomData;

use crate::ffi;

// This is annoying. `vmod_priv` contains a pointer to `vmod_priv_methods`
// that we need to use to free our object, however, we need to piggy-back
// on vmod_priv_methods->free to also clean vmod_priv_methods. So we create
// an InnerVPriv that encapsulate both our object, and vmod_priv_methods.
// Ideally:
// - vmod_priv_methods would have a method to free itself
// - we would be able to create vmod_priv_methods from a const fn
#[derive(Debug)]
pub struct VPriv<'a, T> {
    ptr: &'a mut ffi::vmod_priv,
    phantom: PhantomData<T>,
}

struct InnerVPriv<T> {
    methods: *mut ffi::vmod_priv_methods,
    name: *mut c_char,
    obj: Option<T>,
}

impl<T> VPriv<'_, T> {
    pub unsafe fn from_ptr(vp: *mut ffi::vmod_priv) -> Self {
        Self {
            ptr: vp.as_mut().unwrap(),
            phantom: PhantomData,
        }
    }

    fn get_inner(&mut self) -> Option<&mut InnerVPriv<T>> {
        unsafe { self.ptr.priv_.cast::<InnerVPriv<T>>().as_mut() }
    }

    pub fn store(&mut self, obj: T) {
        if let Some(inner_priv) = self.get_inner() {
            inner_priv.obj = Some(obj);
        } else {
            let name = CString::new(type_name::<T>()).unwrap().into_raw();
            let methods = ffi::vmod_priv_methods {
                magic: ffi::VMOD_PRIV_METHODS_MAGIC,
                type_: name,
                fini: Some(vpriv_free::<T>),
            };
            let methods = Box::into_raw(Box::new(methods));
            let inner_priv = InnerVPriv::<T> {
                methods,
                name,
                obj: Some(obj),
            };
            self.ptr.methods = methods;
            self.ptr.priv_ = Box::into_raw(Box::new(inner_priv)).cast::<c_void>();
        }
    }

    pub fn as_ref(&self) -> Option<&T> {
        let inner = unsafe { self.ptr.priv_.cast::<InnerVPriv<T>>().as_ref()? };
        inner.obj.as_ref()
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        self.get_inner()?.obj.as_mut()
    }

    pub fn take(&mut self) -> Option<T> {
        let inner = self.get_inner()?;
        std::mem::take(&mut inner.obj)
    }

    pub fn clear(&mut self) {
        if let Some(inner_priv) = self.get_inner() {
            inner_priv.obj = None;
        }
    }
}

unsafe extern "C" fn vpriv_free<T>(_: *const ffi::vrt_ctx, ptr: *mut c_void) {
    let inner_priv = Box::from_raw(ptr.cast::<InnerVPriv<T>>());
    drop(CString::from_raw(inner_priv.name));
    drop(Box::from_raw(inner_priv.methods));
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::ptr::null;

    use super::*;

    #[test]
    fn exploration() {
        let mut vp = ffi::vmod_priv::default();
        let mut vpriv_int = unsafe { VPriv::from_ptr(&mut vp) };
        assert_eq!(None, vpriv_int.as_ref());

        let x_in = 5;
        vpriv_int.store(x_in);
        assert_eq!(x_in, *vpriv_int.as_ref().unwrap());

        vpriv_int.store(7);
        assert_eq!(7, *vpriv_int.as_ref().unwrap());

        unsafe {
            assert_eq!(CStr::from_ptr((*vp.methods).type_).to_str().unwrap(), "i32");
            vpriv_free::<i32>(null(), vp.priv_);
        }
    }
}
