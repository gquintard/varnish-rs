struct Obj;
struct ObjVclNameTy;
struct ObjVclNameDup;
struct ObjGen<T> {
    _marker: std::marker::PhantomData<T>,
}

#[varnish::vmod]
mod err {
    use super::*;

    impl super::Obj {}
    impl<T> ObjGen<T> {}

    impl Obj {
        fn non_public() {}

        pub async fn async_fn() {}

        #[event]
        pub fn event_fn() {}
    }

    impl ObjVclNameTy {
        pub fn new(#[vcl_name] a: String) {}
    }

    impl ObjVclNameDup {
        pub fn new(#[vcl_name] a: &str, #[vcl_name] b: &str) {}
    }
}

fn main() {}
