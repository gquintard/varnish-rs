struct ObjNoCtor;

#[varnish::vmod]
mod err {
    use super::ObjNoCtor;
    impl ObjNoCtor {
        pub fn func() -> i64 {
            0
        }
    }
}

fn main() {}
