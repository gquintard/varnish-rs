#![allow(unused_variables)]

use varnish::vmod;

fn main() {}

pub struct PerTask1;
pub struct PerTask2;
pub struct PerVcl1;
pub struct PerVcl2;

#[vmod]
mod tuple {
    use super::{PerTask1, PerTask2, PerVcl1, PerVcl2};

    #[event]
    pub fn on_event(#[shared_per_vcl] vcl_vals: &mut Option<Box<(PerVcl1, PerVcl2)>>) {}

    pub fn per_tsk_val(
        #[shared_per_task] tsk_vals: &mut Option<Box<(PerTask1, PerTask2)>>,
        #[shared_per_vcl] vcl_vals: Option<&(PerVcl1, PerVcl2)>,
    ) {
    }
}
