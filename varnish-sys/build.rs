use std::env;
use std::path::PathBuf;

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

    println!("cargo:rerun-if-env-changed=VARNISH_INCLUDE_PATHS");
    let varnish_paths: Vec<PathBuf> = if let Ok(s) = env::var("VARNISH_INCLUDE_PATHS") {
        // FIXME: If the user has set the VARNISH_INCLUDE_PATHS environment variable, use that.
        //    At the moment we have no way to detect which version it is.
        //    vmod_abi.h  seems to have this line, which can be used in the future.
        //    #define VMOD_ABI_Version "Varnish 7.5.0 eef25264e5ca5f96a77129308edb83ccf84cb1b1"
        s.split(':').map(PathBuf::from).collect()
    } else {
        let pkg = pkg_config::Config::new();
        match pkg.probe("varnishapi") {
            Ok(l) => {
                // version is "7.5.0" and similar
                let mut version = l.version.split('.');
                let major = version
                    .next()
                    .expect("varnishapi invalid version major")
                    .parse::<u32>()
                    .expect("varnishapi invalid version major number");
                let minor = version
                    .next()
                    .expect("varnishapi invalid version minor")
                    .parse::<u32>()
                    .expect("varnishapi invalid version minor number");
                println!("cargo::metadata=version_number={}", l.version);
                if major == 7 && minor <= 5 {
                    println!("cargo::rustc-cfg=feature=\"objcore_in_init\"");
                }
                l.include_paths
            }
            Err(e) => {
                // See https://docs.rs/about/builds#detecting-docsrs
                if env::var("DOCS_RS").is_ok() {
                    eprintln!("libvarnish not found, using saved bindings for the doc.rs: {e}");
                    std::fs::copy("src/bindings.rs.saved", out_path).unwrap();
                    println!("cargo::metadata=version_number=7.6.0");
                    return;
                }
                // FIXME: we should give a URL describing how to install varnishapi
                // I tried to find it, but failed to find a clear URL for this.
                panic!("pkg_config failed to find varnishapi, make sure it is installed: {e:?}");
            }
        }
    };

    // Only link to varnishapi if we found it
    println!("cargo:rustc-link-lib=varnishapi");
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
        // .new_type_alias("VCL_ACL") // *const vrt_acl
        .new_type_alias("VCL_BACKEND") // *const director
        // .new_type_alias("VCL_BLOB") // *const vrt_blob
        // .new_type_alias("VCL_BODY") // *const ::std::os::raw::c_void
        .new_type_alias("VCL_BOOL") // ::std::os::raw::c_uint
        // .new_type_alias("VCL_BYTES") // i64
        .new_type_alias("VCL_DURATION") // VCL_DURATION = vtim_dur = f64
        .new_type_alias("vtim_dur") // VCL_DURATION = vtim_dur = f64
        // .new_type_alias("VCL_ENUM") // *const ::std::os::raw::c_char
        // .new_type_alias("VCL_HEADER") // *const gethdr_s
        // .new_type_alias("VCL_HTTP") // *mut http
        // .new_type_alias("VCL_INSTANCE") // ::std::os::raw::c_void
        .new_type_alias("VCL_INT") // i64
        .new_type_alias("VCL_IP") // *const suckaddr
        .new_type_alias("VCL_PROBE") // *const vrt_backend_probe
        .new_type_alias("VCL_REAL") // f64
        // .new_type_alias("VCL_REGEX") // *const vre
        // .new_type_alias("VCL_STEVEDORE") // *const stevedore
        // .new_type_alias("VCL_STRANDS") // *const strands
        .new_type_alias("VCL_STRING") // *const ::std::os::raw::c_char
        // .new_type_alias("VCL_SUB") // *const vcl_sub
        .new_type_alias("VCL_TIME") // VCL_TIME = vtim_real = f64
        // .new_type_alias("VCL_VCL") // *mut vcl
        // .new_type_alias("VCL_VOID") // ::std::os::raw::c_void
        //
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}
