use std::ffi::c_void;
use std::ptr;
use crate::ffi::vmod_priv;

use crate::vcl::PerVclState;


impl vmod_priv {
    pub unsafe fn take_per_vcl<T>(&mut self) -> Box<PerVclState<T>> {
        if let Some(v) = self.take::<PerVclState<T>>() {
            v
        } else {
            let o = PerVclState::<T>::default();
            Box::new(o)
        }
    }

    /// Use the object as a reference, without taking ownership.
    ///
    /// SAFETY:
    /// * `priv_` must reference a valid `T` object pointer or `NULL`
    /// * `take()` must not be called on the same `vmod_priv` object until the returned reference is dropped
    /// * cleanup must not be done on the object until the returned reference is dropped
    /// * assumes `Box<T>` is equivalent to `&T` when used as a readonly reference, i.e. a box is just a pointer
    pub unsafe fn get_ref<T>(&self) -> Option<&T> {
        self.priv_.cast::<T>().as_ref()
    }
}

#[cfg(not(lts_60))]
mod current_ver {
    use crate::vcl::PerVclState;
    use crate::validate_vrt_ctx;
    use super::get_owned_bbox;
    use std::ffi::c_void;
    use crate::ffi::{vmod_priv, vrt_ctx};
    use crate::ffi::vmod_priv_methods;
    use std::ptr::null;

    /// SAFETY: ensured by Varnish itself
    unsafe impl Sync for vmod_priv_methods {}

    impl vmod_priv {
        /// Transfer ownership of the object to the caller, cleaning up the internal state.
        ///
        /// SAFETY: `priv_` must reference a valid `T` object pointer or `NULL`
        pub unsafe fn take<T>(&mut self) -> Option<Box<T>> {
            // methods does not need to be dropped because `put` always sets it to a static reference
            self.methods = null();
            get_owned_bbox(&mut self.priv_)
        }

        /// Set the object and methods for the `vmod_priv`, and the corresponding static methods.
        ///
        /// SAFETY: The type of `obj` must match the type of the function pointers in `methods`.
        pub unsafe fn put<T>(&mut self, obj: Box<T>, methods: &'static vmod_priv_methods) {
            self.priv_ = Box::into_raw(obj).cast();
            self.methods = methods;
        }

        /// A Varnish callback function to free a `vmod_priv` object.
        /// Here we take the ownership and immediately drop the object of type `T`.
        /// Note that here we get `*priv_` directly, not the `*vmod_priv`
        ///
        /// SAFETY: `priv_` must be a valid pointer to a `T` object or `NULL`.
        pub unsafe extern "C" fn on_fini<T>(_ctx: *const vrt_ctx, mut priv_: *mut c_void) {
            drop(get_owned_bbox::<T>(&mut priv_));
        }

        /// A Varnish callback function to clean up the `PerVclState` object.
        /// Similar to `on_fini`, but also unregisters filters.
        ///
        /// SAFETY: `priv_` must be a valid pointer to a `T` object or `NULL`.
        pub unsafe extern "C" fn on_fini_per_vcl<T>(ctx: *const vrt_ctx, mut priv_: *mut c_void) {
            if let Some(obj) = get_owned_bbox::<PerVclState<T>>(&mut priv_) {
                let PerVclState {
                    mut fetch_filters,
                    mut delivery_filters,
                    user_data,
                } = *obj;
                let ctx = validate_vrt_ctx(ctx);
                ctx.fetch_filters(&mut fetch_filters).unregister_all();
                ctx.delivery_filters(&mut delivery_filters).unregister_all();
                drop(user_data);
            }
        }
    }
}
#[cfg(not(lts_60))]
pub use current_ver::*;

#[cfg(lts_60)]
mod lts_60 {
    use std::ffi::c_void;
    use super::get_owned_bbox;
    use crate::vcl::PerVclState;
    use crate::ffi::{vmod_priv, vrt_ctx};
    use crate::ffi::vmod_priv_free_f;

    impl vmod_priv {
        /// Transfer ownership of the object to the caller, cleaning up the internal state.
        ///
        /// SAFETY: `priv_` must reference a valid `T` object pointer or `NULL`
        pub unsafe fn take<T>(&mut self) -> Option<Box<T>> {
            get_owned_bbox(&mut self.priv_)
        }

        /// Set the object and methods for the `vmod_priv`, and the corresponding static methods.
        ///
        /// SAFETY: The type of `obj` must match the type of the function pointers in `methods`.
        pub unsafe fn put<T>(&mut self, obj: Box<T>, free_method: vmod_priv_free_f) {
            self.priv_ = Box::into_raw(obj).cast();
            self.free = free_method;
        }

        /// A Varnish callback function to free a `vmod_priv` object.
        /// Here we take the ownership and immediately drop the object of type `T`.
        /// Note that here we get `*priv_` directly, not the `*vmod_priv`
        ///
        /// SAFETY: `priv_` must be a valid pointer to a `T` object or `NULL`.
        pub unsafe extern "C" fn on_fini<T>(mut priv_: *mut c_void) {
            drop(get_owned_bbox::<T>(&mut priv_));
        }

        /// A Varnish callback function to clean up the `PerVclState` object.
        /// Similar to `on_fini`, but also unregisters filters.
        ///
        /// SAFETY: `priv_` must be a valid pointer to a `T` object or `NULL`.
        pub unsafe extern "C" fn on_fini_per_vcl<T>(mut priv_: *mut c_void) {
            if let Some(obj) = get_owned_bbox::<PerVclState<T>>(&mut priv_) {
                let PerVclState {
                    user_data,
                } = *obj;
                drop(user_data);
            }
        }
    }
}

#[cfg(lts_60)]
pub use lts_60::*;

/// Take ownership of the object of type `T` and return it as a `Box<T>`.
/// The original pointer is set to null.
///
/// SAFETY: `priv_` must reference a valid `T` object pointer or `NULL`
unsafe fn get_owned_bbox<T>(priv_: &mut *mut c_void) -> Option<Box<T>> {
    let obj = ptr::replace(priv_, ptr::null_mut());
    if obj.is_null() {
        None
    } else {
        Some(Box::from_raw(obj.cast::<T>()))
    }
}
