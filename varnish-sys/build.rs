use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=varnishapi");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

    let vapi = match pkg_config::Config::new()
        .atleast_version("7.0")
        .probe("varnishapi") {
            Ok(l) => l,
            Err(e) => {
                println!("no system libvarnish found, using the pre-generated bindings {}", e);
                std::fs::copy("src/bindings.rs.saved", out_path).unwrap();
                return;
            }
        };

    println!("cargo:rerun-if-changed=src/wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .blocklist_item("FP_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_args(
            vapi.include_paths
                .iter()
                .map(|i| format!("-I{}", i.to_str().unwrap())),
        )
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}
