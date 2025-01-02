use varnish::ffi::VCL_BACKEND;

varnish::run_vtc_tests!("tests/*.vtc");

/// kv only contains one element: a String->String hashmap that can be used in parallel
#[allow(non_camel_case_types)]
#[derive(Default)]
pub struct pool {
    storage: std::sync::Mutex<Vec<VCL_BACKEND>>,
}

/// A simple string dictionary in your VCL
#[varnish::vmod(docs = "API.md")]
mod director {
    use rand; 
    use varnish::ffi::VCL_BACKEND;

    use super::pool;

    impl pool {
        pub fn new() -> Self {
            Self { ..Default::default() }
        }

        pub fn add_backend(&self, be: VCL_BACKEND) {
            let mut pool = self.storage.lock().unwrap();
            pool.push(be);
        }

        pub fn backend(&self) -> VCL_BACKEND {
            let pool = self.storage.lock().unwrap();
            // this is not evenly distributed, but this isn't the focus of this vmod
            let i = rand::random::<usize>() % pool.len();
            pool[i]
        }
    }
}
