#![allow(non_camel_case_types)]

varnish::boilerplate!();

use varnish::vcl::backend::{Backend, Serve, Transfer, VCLBackendPtr};
use varnish::vcl::ctx::Ctx;
use varnish::vcl::Result;

varnish::vtc!(test01);

// parrot is our VCL object, which just holds a rust Backend,
// it only needs two functions:
// - new(), so that the VCL can instantiate it
// - backend(), so that we can produce a C pointer for varnish to use
pub struct parrot {
    be: Backend<Sentence, SentenceTransfer>,
}

impl parrot {
    pub fn new(ctx: &mut Ctx, vcl_name: &str, to_repeat: &str) -> Result<Self> {
        // to create the backend, we need:
        // - the vcl context, that we just pass along
        // - the vcl_name (how the vcl writer named the object)
        // - an 
        let be = Backend::new(
                ctx,
                vcl_name,
                Sentence {
                    v: Vec::from(to_repeat),
                },
            )?;

        Ok(parrot { be })
    }

    pub fn backend(&self, _ctx: &Ctx) -> VCLBackendPtr {
        self.be.vcl_ptr()
    }
}

pub struct Sentence {
    v: Vec<u8>,
}

impl Serve<SentenceTransfer> for Sentence {
    fn get_type(&self) -> String {
        "parrot".to_string()
    }

    fn get_headers(&self, _ctx: &mut Ctx) -> Result<Option<SentenceTransfer>> {
        Ok(Some(SentenceTransfer {
            v: self.v.clone(),
            cursor: 0,
        }))
    }
}

pub struct SentenceTransfer {
    v: Vec<u8>,
    cursor: usize,
}

impl Transfer for SentenceTransfer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // don't worry about what we already transfered
        let remaining = &self.v[self.cursor..];

        // can't send more than what we have, or more than what the buffer can hold
        let l = std::cmp::min(remaining.len(), buf.len());

        // copy data into the buffer
        for (p, val) in std::iter::zip(buf, remaining) {
            *p = *val;
        }

        // move the buffer for nex time
        self.cursor += l;

        // everything went fine, we copied l bytes into buf
        Ok(l)
    }

    // we know from the start how much we'll send
    fn len(&self) -> Option<usize> {
        Some(self.v.len())
    }
}
