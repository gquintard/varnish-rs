use crate::ffi::director;
use crate::vcl::Serve;

/// Return the private pointer as a reference to the [`Serve`] object
/// FIXME: should it return a `&mut` instead?
pub fn get_backend<S: Serve>(v: &director) -> &S {
    unsafe { v.priv_.cast::<S>().as_ref().unwrap() }
}
