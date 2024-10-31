#![allow(unused_variables)]
#![allow(non_camel_case_types)]

use varnish::vmod;

fn main() {}

#[vmod]
mod obj {
    use varnish::vcl::Ctx;

    pub struct kv1;
    impl kv1 {
        pub fn new(cap: Option<i64>) -> Self {
            kv1
        }
        pub fn set(&self, key: &str, value: &str) {}
        pub fn get(&self, key: &str) -> String {
            String::default()
        }
    }

    pub struct kv2;
    impl kv2 {
        pub fn new(cap: Option<i64>, #[vcl_name] name: &str) -> Self {
            kv2
        }
        pub fn set(&self, key: &str, value: Option<&str>) {}
    }

    pub struct kv3;
    impl kv3 {
        pub fn new(ctx: &mut Ctx, cap: Option<i64>, #[vcl_name] name: &str) -> Self {
            kv3
        }
        pub fn set(&self, ctx: &mut Ctx, key: &str, value: Option<&str>) {}
    }
}
