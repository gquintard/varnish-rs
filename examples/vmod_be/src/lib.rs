use varnish::vcl::{Ctx, Serve, Transfer, VclError};

varnish::run_vtc_tests!("tests/*.vtc");

/// a simple STRING dictionary in your VCL
#[varnish::vmod(docs = "README.md")]
mod be {
    use varnish::ffi::VCL_BACKEND;
    use varnish::vcl::{Backend, Ctx, VclError};

    use super::{Body, Sentence};

    /// parrot is our VCL object, which just holds a rust Backend,
    /// it only needs two functions:
    /// - new(), so that the VCL can instantiate it
    /// - backend(), so that we can produce a C pointer for varnish to use
    #[allow(non_camel_case_types)]
    pub struct parrot {
        be: Backend<Sentence, Body>,
    }

    impl parrot {
        pub fn new(
            ctx: &mut Ctx,
            // Varnish automatically supplies this parameter if listed here
            // It is not part of the object instantiation in VCL
            #[vcl_name] name: &str,
            to_repeat: &str,
        ) -> Result<Self, VclError> {
            // to create the backend, we need:
            // - the vcl context, that we just pass along
            // - the vcl_name (how the vcl writer named the object)
            // - a struct that implements the Serve trait
            let be = Backend::new(
                ctx,
                name,
                Sentence {
                    v: Vec::from(to_repeat),
                },
                false,
            )?;

            Ok(parrot { be })
        }

        pub fn backend(&self) -> VCL_BACKEND {
            self.be.vcl_ptr()
        }
    }
}

// Sentence is just a Vec<u8> holding the string we were asked to repeat
pub struct Sentence {
    v: Vec<u8>,
}

// a lot of the Serve trait's methods are optional, but we need to implement
// - get_type() for debugging reasons when something fails
// - get_headers() that actually builds the response headers,
//   and returns a Body
impl Serve<Body> for Sentence {
    fn get_type(&self) -> &str {
        "parrot"
    }

    fn get_headers(&self, ctx: &mut Ctx) -> Result<Option<Body>, VclError> {
        let beresp = ctx.http_beresp.as_mut().unwrap();
        beresp.set_status(200);
        beresp.set_header("server", "parrot")?;

        Ok(Some(Body {
            p: self.v.as_ptr(),
            left: self.v.len(),
        }))
    }
}

// it's not great to be passing pointers around, but that save us from copying
// the vector or from using a mutex/arc, and we know the Vec will survive the
// transfer
pub struct Body {
    p: *const u8,
    left: usize,
}

impl Transfer for Body {
    // Varnish will call us over and over, asking us to fill buffers
    // we'll happily oblige by filling as much as we can every time
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, VclError> {
        // can't send more than what we have, or more than what the buffer can hold
        let l = std::cmp::min(self.left, buf.len());

        // there's no way for the compiler to prove that self.p isn't dangling
        // at this stage, so we'll ask it to trust us as we rebuild the slice
        // it points to
        let to_send = unsafe { std::slice::from_raw_parts(self.p, l) };

        // copy data into the buffer
        for (p, val) in std::iter::zip(buf, to_send) {
            *p = *val;
        }

        // increment the pointer and decrease left for next time
        // and once again, we must ask the compiler to trust us as pointer
        // arithmetic is dangerous
        unsafe {
            self.p = self.p.add(l);
        }
        self.left -= l;

        // everything went fine, we copied l bytes into buf
        Ok(l)
    }

    // we know from the start how much we'll send
    fn len(&self) -> Option<usize> {
        Some(self.left)
    }
}
