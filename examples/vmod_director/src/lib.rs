use varnish::ffi::VCL_BACKEND;

varnish::run_vtc_tests!("tests/*.vtc");

/// kv only contains one element: a String->String hashmap that can be used in parallel
#[allow(non_camel_case_types)]
#[derive(Default)]
pub struct pool {
    storage: std::sync::Mutex<Vec<VCL_BACKEND>>,
}

/// A simple varnish director
#[varnish::vmod(docs = "API.md")]
mod director {
    use rand;
    use varnish::ffi::{VCL_BACKEND, VRT_BACKEND_MAGIC, vrt_backend};
    use varnish::vcl::VclError;
    use std::ffi::CStr;

    use super::pool;

    impl pool {
        pub fn new() -> Self {
            Self { ..Default::default() }
        }

        pub fn add_backend(&self, be: VCL_BACKEND) -> Result<(), VclError> {
            unsafe {
                let name_ptr = (*be.0).vcl_name;
                let name = CStr::from_ptr(name_ptr)
                    .to_str()
                    .map(String::from)
                    .unwrap();
                println!("backend name: {:?}", name);
            }

            unsafe {
                let backend = (*be.0).priv_ as *const vrt_backend;
                if (*backend).magic == VRT_BACKEND_MAGIC {
                    println!("backend magic was correct");
                } else {
                    println!("expected magic: {:x}, got magic: {:x}", VRT_BACKEND_MAGIC, (*backend).magic);
                    return Err(VclError::new("Invalid VRT_BACKEND_MAGIC".to_string()));
                }
            }

            let mut pool = self.storage.lock().unwrap();
            pool.push(be);

            Ok(())
        }

        pub fn backend(&self) -> VCL_BACKEND {
            let pool = self.storage.lock().unwrap();
            // this is not evenly distributed, but this isn't the focus of this vmod
            let i = rand::random::<usize>() % pool.len();
            pool[i]
        }
    }
}

#[cfg(test)]
mod test {
    varnish::run_vtc_tests!("tests/*.vtc", true);
}
