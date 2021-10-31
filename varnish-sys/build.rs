use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=varnishapi");
    println!("cargo:rerun-if-changed=wrapper.h");

    let vapi = pkg_config::Config::new()
        .atleast_version("7.0")
        .probe("varnishapi")
        .unwrap();

    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .blacklist_item("FP_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_args(
            vapi.include_paths
                .iter()
                .map(|i| format!("-I{}", i.to_str().unwrap())),
        )
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
