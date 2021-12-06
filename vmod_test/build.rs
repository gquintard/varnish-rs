// before we actually compile our code, parse `vmod.vcc` to generate some boilerplate
fn main() {
    varnish::generate_boilerplate().unwrap();
}
