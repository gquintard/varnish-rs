use varnish_sys::VMOD_ABI_Version;

fn main() {
    if VMOD_ABI_Version.starts_with(b"Varnish Plus") {
        println!("cargo:rustc-cfg=varnish_plus");
    }
}
