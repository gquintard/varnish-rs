use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=varnishapi");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

    println!("cargo:rerun-if-env-changed=VARNISH_INCLUDE_PATHS");
    let varnish_paths: Vec<PathBuf> = match env::var("VARNISH_INCLUDE_PATHS") {
        Ok(s) => s.split(':').map(PathBuf::from).collect(),
        Err(_) => {
            match pkg_config::Config::new()
                .atleast_version("7.5")
                .probe("varnishapi")
            {
                Ok(l) => l.include_paths,
                Err(e) => {
                    println!("no system libvarnish found, using the pre-generated bindings {e}");
                    std::fs::copy("src/bindings.rs.saved", out_path).unwrap();
                    return;
                }
            }
        }
    };

    println!("cargo:rerun-if-changed=src/wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .blocklist_item("FP_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .clang_args(
            varnish_paths
                .iter()
                .map(|i| format!("-I{}", i.to_str().unwrap())),
        )
        .ctypes_prefix("::std::ffi")
        .derive_copy(true)
        .derive_debug(true)
        .derive_default(true)
        .generate_cstr(true)
        //
        // VCL_ACL = *const vrt_acl
        // VCL_BACKEND = *const director
        // VCL_BLOB = *const vrt_blob
        // VCL_BODY = *const ::std::os::raw::c_void
        // VCL_BOOL = ::std::os::raw::c_uint
        .new_type_alias_deref("VCL_BOOL")
        // VCL_BYTES = i64
        // VCL_DURATION = vtim_dur
        // VCL_ENUM = *const ::std::os::raw::c_char
        // VCL_HEADER = *const gethdr_s
        // VCL_HTTP = *mut http
        // VCL_INSTANCE = ::std::os::raw::c_void
        // VCL_INT = i64
        // VCL_IP = *const suckaddr
        // VCL_PROBE = *const vrt_backend_probe
        // VCL_REAL = f64
        // VCL_REGEX = *const vre
        // VCL_STEVEDORE = *const stevedore
        // VCL_STRANDS = *const strands
        // VCL_STRING = *const ::std::os::raw::c_char
        // VCL_SUB = *const vcl_sub
        // VCL_TIME = vtim_real
        // VCL_VCL = *mut vcl
        // VCL_VOID = ::std::os::raw::c_void
        //
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}
