#![allow(unused_variables)]

use varnish::vmod;

fn main() {}

pub struct PerTask;
pub struct PerVcl;

#[vmod]
mod task {
    use super::{PerTask, PerVcl};
    use varnish::vcl::{Ctx, Event};

    #[event]
    pub fn on_event(evt: Event, ctx: &mut Ctx, #[shared_per_vcl] vcl: &mut Option<Box<PerVcl>>) {
        panic!()
    }

    pub fn per_vcl_val(#[shared_per_vcl] vcl: Option<&PerVcl>) {
        panic!()
    }

    pub fn per_vcl_opt(#[shared_per_vcl] vcl: Option<&PerVcl>, op: Option<i64>) {
        panic!()
    }

    impl PerVcl {
        pub fn new(#[shared_per_vcl] vcl: &mut Option<Box<PerVcl>>) -> Self {
            panic!()
        }

        pub fn both(
            &self,
            #[shared_per_task] tsk: &mut Option<Box<PerTask>>,
            #[shared_per_vcl] vcl: Option<&PerVcl>,
        ) {
            panic!()
        }

        pub fn both_pos(
            &self,
            #[shared_per_task] tsk: &mut Option<Box<PerTask>>,
            #[shared_per_vcl] vcl: Option<&PerVcl>,
            val: i64,
        ) {
            panic!()
        }

        pub fn both_opt(
            &self,
            #[shared_per_task] tsk: &mut Option<Box<PerTask>>,
            #[shared_per_vcl] vcl: Option<&PerVcl>,
            opt: Option<i64>,
        ) {
            panic!()
        }
    }

    pub fn per_tsk_val(#[shared_per_task] tsk: &mut Option<Box<PerTask>>) {
        panic!()
    }

    pub fn per_tsk_opt(#[shared_per_task] tsk: &mut Option<Box<PerTask>>, op: Option<i64>) {
        panic!()
    }
}
