varnish::run_vtc_tests!("tests/*.vtc");

/// A simple string dictionary in your VCL
#[varnish::vmod(docs = "README.md")]
mod object {
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// kv only contains one element: a mutex wrapping a String->String hashmap
    #[allow(non_camel_case_types)]
    pub struct kv {
        mutexed_hash_map: Mutex<HashMap<String, String>>,
    }

    // implementation needs the same methods as defined in the vcc, plus "new()"
    // corresponding to the constructor, which requires the context (_ctx) , and the
    // name of the object in VLC (_vcl_name)
    impl kv {
        /// Create a new key-value store, with an optional capacity.
        /// If `cap` is 0 or less, it will be ignored.
        pub fn new(cap: Option<i64>) -> Self {
            // depending on whether cap was actually passed, and on its value,
            // call a different constructor
            let h = match cap {
                None => HashMap::new(),
                Some(n) if n <= 0 => HashMap::new(),
                Some(n) => HashMap::with_capacity(n as usize),
            };

            Self {
                mutexed_hash_map: Mutex::new(h),
            }
        }

        /// Retrieve the value associated `key`, or an empty string if `key` didn't exist.
        ///
        /// There is currently an inefficiency in the implementation: when found, the value
        /// is a `&str`, so it must be converted to a `String` to be returned, which is then
        /// copied into an internal Varnish workspace. In the future, we will provide a way
        /// to avoid this double-copy.
        pub fn get(&self, key: &str) -> String {
            self.mutexed_hash_map // access our member field
                .lock() // lock the mutex to access the hashmap
                .unwrap() // panic if unlocking went wrong
                .get(key) // look for key
                .map_or(String::new(), |v| v.to_string()) // used EMPTY_STRING if key isn't found
        }

        /// Insert a key-value pair into the store.
        pub fn set(&self, key: &str, value: &str) {
            self.mutexed_hash_map
                .lock()
                .unwrap()
                .insert(key.to_owned(), value.to_owned());
        }
    }
}
