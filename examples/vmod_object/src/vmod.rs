// prevent the compiler warnings because it'd prefer `kv` to be named `Kv` instead
#![allow(non_camel_case_types)]

use varnish::vcl::ctx::Ctx;
use std::collections::HashMap;
use varnish::vcl::convert::IntoVCL;
use std::sync::Mutex;
use varnish_sys::VCL_STRING;

#[cfg(test)]
varnish::vtc!(test01);

const EMPTY_STRING: String = String::new();

// kv only contains one element: a mutex wrapping a String->String hashmap
pub struct kv {
    mutexed_hash_map: Mutex<HashMap<String, String>>,
}

// implementation needs the same methods as defined in the vcc, plus "new()"
// corresponding to the constructor, which requires the context (_ctx) , and the
// name of the object in VLC (_vcl_name)
impl kv {
    // constructor doesn't need a Ctx, or the VCL name, hence the _ prefix
    pub fn new(_ctx: &Ctx, _vcl_name: &str, cap: Option<i64>) -> Self {
        let h = match cap {
            None => HashMap::new(),
            Some(n) if n <= 0 => HashMap::new(),
            Some(n) => HashMap::with_capacity(n as usize),
        };
        kv {
            mutexed_hash_map: Mutex::new(h),
        }
    }

    // to be more efficient and avoid duplicating the string result just to
    // pass it to the boilerplate, we can do the conversion to a VCL ourselves
    pub fn get(&self, ctx: &mut Ctx, key: &str) -> VCL_STRING {
        self.mutexed_hash_map // access our member field
            .lock() // lock the mutex to access the hashmap
            .unwrap() // panic if unlocking went wrong
            .get(key) // look for key
            .unwrap_or(&EMPTY_STRING) // used EMPTY_STRING if key isn't found
            .as_str() // make it an &str
            .into_vcl(&mut ctx.ws) // copy the key before returning it
    }

    pub fn set(&self, _: &Ctx, key: &str, value: &str) {
        self.mutexed_hash_map
            .lock()
            .unwrap()
            .insert(key.to_owned(), value.to_owned());
    }
}
