// before we actually compile our code, parse `vmod.vcc` to generate some boilerplate
fn main() {
    varnish::vmodtool::generate().unwrap();
    #[cfg(docsrs)]
    panic!("panic!");
}
