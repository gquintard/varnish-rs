use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;

use varnish_sys;

pub struct VPriv<T> {
    ptr: *mut varnish_sys::vmod_priv,
    phantom: PhantomData<T>,
}

struct InnerVPriv<T> {
    obj: Option<T>,
}

impl<T> VPriv<T> {
    pub fn new(vp: *mut varnish_sys::vmod_priv) -> Self {
        assert_ne!(vp, ptr::null_mut());
        VPriv::<T> {
            ptr: vp,
            phantom: PhantomData,
        }
    }

    fn get_inner(&mut self) -> Option<&mut InnerVPriv<T>> {
        unsafe { (self.ptr.as_mut()?.priv_ as *mut InnerVPriv<T>).as_mut() }
    }

    pub fn store(&mut self, obj: T) {
        unsafe {
            if self.get_inner().is_none() {
                let inner_priv: InnerVPriv<T> = InnerVPriv { obj: None };

                (*self.ptr).priv_ = Box::into_raw(Box::new(inner_priv)) as *mut c_void;
                (*self.ptr).free = Some(vpriv_free::<T>);
            }
        }
        let inner_priv = self.get_inner().unwrap();
        inner_priv.obj = Some(obj);
    }

    pub fn as_ref(&self) -> Option<&T> {
        let inner = unsafe { ((*self.ptr).priv_ as *mut InnerVPriv<T>).as_ref()? };
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

unsafe extern "C" fn vpriv_free<T>(ptr: *mut c_void) {
    drop(Box::from_raw(ptr as *mut InnerVPriv<T>));
}

#[test]
fn exploration() {
    let mut vp = varnish_sys::vmod_priv {
        priv_: ptr::null::<c_void>() as *mut c_void,
        len: 0,
        free: Some(vpriv_free::<i32>),
    };

    let mut vpriv_int = VPriv::new(&mut vp);
    assert_eq!(None, vpriv_int.as_ref());

    let x_in = 5;
    vpriv_int.store(x_in);
    assert_eq!(x_in, *vpriv_int.as_ref().unwrap());

    vpriv_int.store(7);
    assert_eq!(7, *vpriv_int.as_ref().unwrap());

    unsafe {
        vpriv_free::<i32>(vp.priv_);
    }
}
