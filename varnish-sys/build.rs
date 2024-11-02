use bindgen_helpers::{rename_enum, Renamer};
use std::path::PathBuf;
use std::{env, fs};

fn main() {
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

    println!("cargo:rerun-if-env-changed=VARNISH_INCLUDE_PATHS");
    let Some(varnish_paths) = find_include_dir(&out_path) else {
        return;
    };

    let mut ren = Renamer::default();
    rename_enum!(ren, "VSL_tag_e" => "VslTag", prefix: "SLT_"); // SLT_Debug
    rename_enum!(ren, "boc_state_e" => "BocState", prefix: "BOS_"); // BOS_INVALID
    rename_enum!(ren, "director_state_e" => "DirectorState", prefix: "DIR_S_", "HDRS" => "Headers"); // DIR_S_NULL
    rename_enum!(ren, "gethdr_e" => "GetHeader", prefix: "HDR_"); // HDR_REQ_TOP
    rename_enum!(ren, "sess_attr" => "SessionAttr", prefix: "SA_"); // SA_TRANSPORT
    rename_enum!(ren, "lbody_e" => "Body", prefix: "LBODY_"); // LBODY_SET_STRING
    rename_enum!(ren, "task_prio" => "TaskPriority", prefix: "TASK_QUEUE_"); // TASK_QUEUE_BO
    rename_enum!(ren, "vas_e" => "Vas", prefix: "VAS_"); // VAS_WRONG
    rename_enum!(ren, "vcl_event_e" => "VclEvent", prefix: "VCL_EVENT_"); // VCL_EVENT_LOAD
    rename_enum!(ren, "vcl_func_call_e" => "VclFuncCall", prefix: "VSUB_"); // VSUB_STATIC
    rename_enum!(ren, "vcl_func_fail_e" => "VclFuncFail", prefix: "VSUB_E_"); // VSUB_E_OK
    rename_enum!(ren, "vdp_action" => "VdpAction", prefix: "VDP_"); // VDP_NULL
    rename_enum!(ren, "vfp_status" => "VfpStatus", prefix: "VFP_"); // VFP_ERROR

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
        // These two types are set to `c_void`, which is not copyable.
        // Plus the new wrapped empty type might be pointless... or not?
        .type_alias("VCL_VOID")
        .type_alias("VCL_INSTANCE")
        .new_type_alias("VCL_.*")
        .new_type_alias("vtim_.*") // VCL_DURATION = vtim_dur = f64
        //
        // FIXME: some enums should probably be done as rustified_enum (exhaustive)
        .rustified_non_exhaustive_enum(ren.get_regex_str())
        .parse_callbacks(Box::new(ren))
        //
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    bindings
        .write_to_file(&out_path)
        .expect("Couldn't write bindings!");

    // Compare generated `out_path` file to the checked-in `bindings.for-docs` file,
    // and if they differ, raise a warning.
    let generated = fs::read_to_string(&out_path).unwrap();
    let checked_in = fs::read_to_string("bindings.for-docs").unwrap();
    if generated != checked_in {
        println!(
            "cargo::warning=Generated bindings differ from checked-in bindings.for-docs. Update with   cp {} varnish-sys/bindings.for-docs",
            out_path.display()
        );
    }
}

fn find_include_dir(out_path: &PathBuf) -> Option<Vec<PathBuf>> {
    if let Ok(s) = env::var("VARNISH_INCLUDE_PATHS") {
        // FIXME: If the user has set the VARNISH_INCLUDE_PATHS environment variable, use that.
        //    At the moment we have no way to detect which version it is.
        //    vmod_abi.h  seems to have this line, which can be used in the future.
        //    #define VMOD_ABI_Version "Varnish 7.5.0 eef25264e5ca5f96a77129308edb83ccf84cb1b1"
        return Some(s.split(':').map(PathBuf::from).collect());
    }

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
                println!("cargo::rustc-cfg=feature=\"_objcore_in_init\"");
            }
            Some(l.include_paths)
        }
        Err(e) => {
            // See https://docs.rs/about/builds#detecting-docsrs
            if env::var("DOCS_RS").is_ok() {
                eprintln!("libvarnish not found, using saved bindings for the doc.rs: {e}");
                fs::copy("bindings.for-docs", out_path).unwrap();
                println!("cargo::metadata=version_number=7.6.0");
                None
            } else {
                // FIXME: we should give a URL describing how to install varnishapi
                // I tried to find it, but failed to find a clear URL for this.
                panic!("pkg_config failed to find varnishapi, make sure it is installed: {e:?}");
            }
        }
    }
}
