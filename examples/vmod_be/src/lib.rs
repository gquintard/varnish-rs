use varnish::vcl::{Backend, Body, Ctx, Serve, VclError};

varnish::run_vtc_tests!("tests/*.vtc");

#[allow(non_camel_case_types)]
struct parrot {
    backend: Backend<Sentence>,
}

/// a simple STRING dictionary in your VCL
#[varnish::vmod(docs = "README.md")]
mod be {
    use varnish::vcl::{Backend, BackendHandle, Ctx, VclError};

    use super::{parrot, Sentence};

    /// parrot is our VCL object, which just holds a rust Backend,
    /// it only needs two functions:
    /// - new(), so that the VCL can instantiate it
    /// - backend(), so that we can produce a C pointer for varnish to use
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
            let backend = Backend::new(
                ctx,
                "parrot",
                name,
                Sentence {
                    data: Vec::from(to_repeat),
                },
                false,
            )?;

            Ok(parrot { backend })
        }

        pub fn backend(&self) -> &BackendHandle {
            &self.backend.handle
        }
    }
}

// Sentence is just a Vec<u8> holding the string we were asked to repeat
struct Sentence {
    data: Vec<u8>,
}

// a lot of the Serve trait's methods are optional, but we do need to implement
// get_headers() that sets the response headers, and returns a Body
impl Serve for Sentence {
    fn get_headers(&self, ctx: &mut Ctx) -> Result<Body, VclError> {
        let beresp = ctx.http_beresp.as_mut().unwrap();
        beresp.set_status(200);
        beresp.set_header("server", "parrot")?;

        Ok(Body::Buffer(Box::new(self.data.clone())))
    }
}
