#![allow(unused_variables)]
#![allow(non_camel_case_types)]

use varnish::vmod;

fn main() {}

#[vmod]
mod obj {
    pub struct kv1;
    impl kv1 {
        pub fn new(cap: Option<i64>) -> Self {
            panic!()
        }
        pub fn set(&self, key: &str, value: &str) {
            panic!()
        }
        pub fn get(&self, key: &str) -> String {
            panic!()
        }
    }

    pub struct kv2;
    impl kv2 {
        pub fn new(cap: Option<i64>, #[vcl_name] name: &str) -> Self {
            panic!()
        }
        pub fn set(&self, key: &str, value: Option<&str>) {
            panic!()
        }
    }
}
