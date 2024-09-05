use std::any::type_name;
#[cfg(test)]
use std::ffi::CStr;
use std::ffi::{c_void, CString};
use std::marker::PhantomData;
use std::ptr;

use crate::ffi;

// This is annoying. `vmod_priv` contains a pointer to `vmod_priv_methods`
// that we need to use to free our object, however, we need to piggy-back
// on vmod_priv_methods->free to also clean vmod_priv_methods. So we create
// an InnerVPriv that encapsulate both our object, and vmod_priv_methods.
// Ideally:
// - vmod_priv_methods would have a method to free itself
// - we would be able to create vmod_priv_methods from a const fn
pub struct VPriv<T> {
    ptr: *mut ffi::vmod_priv,
    phantom: PhantomData<T>,
}

struct InnerVPriv<T> {
    methods: *mut ffi::vmod_priv_methods,
    name: *mut CString,
    obj: Option<T>,
}

impl<T> VPriv<T> {
    pub fn new(vp: *mut ffi::vmod_priv) -> Self {
        assert_ne!(vp, ptr::null_mut());
        VPriv::<T> {
            ptr: vp,
            phantom: PhantomData,
        }
    }

    fn get_inner(&mut self) -> Option<&mut InnerVPriv<T>> {
        unsafe { self.ptr.as_mut()?.priv_.cast::<InnerVPriv<T>>().as_mut() }
    }

    pub fn store(&mut self, obj: T) {
        unsafe {
            if self.get_inner().is_none() {
                let name = Box::into_raw(Box::new(CString::new(type_name::<T>()).unwrap()));
                let methods = ffi::vmod_priv_methods {
                    magic: ffi::VMOD_PRIV_METHODS_MAGIC,
                    type_: (*name).as_ptr(),
                    fini: Some(vpriv_free::<T>),
                };

                let methods_ptr = Box::into_raw(Box::new(methods));
                let inner_priv: InnerVPriv<T> = InnerVPriv {
                    methods: methods_ptr,
                    name,
                    obj: None,
                };
                (*self.ptr).methods = methods_ptr;
                (*self.ptr).priv_ = Box::into_raw(Box::new(inner_priv)).cast::<c_void>();
            }
        }
        let inner_priv = self.get_inner().unwrap();
        inner_priv.obj = Some(obj);
    }

    pub fn as_ref(&self) -> Option<&T> {
        let inner = unsafe { (*self.ptr).priv_.cast::<InnerVPriv<T>>().as_ref()? };
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
    drop(Box::from_raw(inner_priv.name));
    drop(Box::from_raw(inner_priv.methods));
}

#[test]
fn exploration() {
    let mut vp = ffi::vmod_priv {
        priv_: ptr::null::<c_void>() as *mut c_void,
        len: 0,
        methods: ptr::null::<ffi::vmod_priv_methods>() as *mut ffi::vmod_priv_methods,
    };

    let mut vpriv_int = VPriv::new(&mut vp);
    assert_eq!(None, vpriv_int.as_ref());

    let x_in = 5;
    vpriv_int.store(x_in);
    assert_eq!(x_in, *vpriv_int.as_ref().unwrap());

    vpriv_int.store(7);
    assert_eq!(7, *vpriv_int.as_ref().unwrap());

    unsafe {
        assert_eq!(CStr::from_ptr((*vp.methods).type_).to_str().unwrap(), "i32");

        vpriv_free::<i32>(ptr::null::<ffi::vrt_ctx>(), vp.priv_);
    }
}
