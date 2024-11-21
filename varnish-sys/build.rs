use std::path::PathBuf;
use std::str::Split;
use std::{env, fs};

use bindgen_helpers::{rename_enum, Renamer};

static BINDINGS_FILE: &str = "bindings.for-docs";
static BINDINGS_FILE_VER: &str = "7.6.1";

fn main() {
    // <=7.5 passed *objcore in vdp_init_f as the 4th param
    println!("cargo::rustc-check-cfg=cfg(varnishsys_objcore_in_init)");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

    println!("cargo:rerun-if-env-changed=VARNISH_INCLUDE_PATHS");
    let Some((varnish_paths, varnish_ver)) = find_include_dir(&out_path) else {
        return;
    };

    let mut ren = Renamer::default();
    rename_enum!(ren, "VSL_tag_e" => "VslTag", remove: "SLT_"); // SLT_Debug
    rename_enum!(ren, "boc_state_e" => "BocState", remove: "BOS_"); // BOS_INVALID
    rename_enum!(ren, "director_state_e" => "DirectorState", remove: "DIR_S_", "HDRS" => "Headers"); // DIR_S_NULL
    rename_enum!(ren, "gethdr_e" => "GetHeader", remove: "HDR_"); // HDR_REQ_TOP
    rename_enum!(ren, "sess_attr" => "SessionAttr", remove: "SA_"); // SA_TRANSPORT
    rename_enum!(ren, "lbody_e" => "Body", remove: "LBODY_"); // LBODY_SET_STRING
    rename_enum!(ren, "task_prio" => "TaskPriority", remove: "TASK_QUEUE_"); // TASK_QUEUE_BO
    rename_enum!(ren, "vas_e" => "Vas", remove: "VAS_"); // VAS_WRONG
    rename_enum!(ren, "vcl_event_e" => "VclEvent", remove: "V(CL|DI)_EVENT_"); // VCL_EVENT_LOAD
    rename_enum!(ren, "vcl_func_call_e" => "VclFuncCall", remove: "VSUB_"); // VSUB_STATIC
    rename_enum!(ren, "vcl_func_fail_e" => "VclFuncFail", remove: "VSUB_E_"); // VSUB_E_OK
    rename_enum!(ren, "vdp_action" => "VdpAction", remove: "VDP_"); // VDP_NULL
    rename_enum!(ren, "vfp_status" => "VfpStatus", remove: "VFP_"); // VFP_ERROR

    println!("cargo:rustc-link-lib=varnishapi");
    println!("cargo:rerun-if-changed=src/wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .blocklist_item("FP_.*")
        .blocklist_item("FILE")
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
    let checked_in = fs::read_to_string(BINDINGS_FILE).unwrap_or_default();
    if generated != checked_in {
        println!(
            "cargo::warning=Generated bindings from Varnish {varnish_ver} differ from checked-in {BINDINGS_FILE}. Update with   cp {} varnish-sys/{BINDINGS_FILE}",
            out_path.display()
        );
    } else if BINDINGS_FILE_VER != varnish_ver {
        println!(
            r#"cargo::warning=Generated bindings **version** from Varnish {varnish_ver} differ from checked-in {BINDINGS_FILE}. Update `build.rs` file with   BINDINGS_FILE_VER = "{varnish_ver}""#
        );
    }
}

fn find_include_dir(out_path: &PathBuf) -> Option<(Vec<PathBuf>, String)> {
    if let Ok(s) = env::var("VARNISH_INCLUDE_PATHS") {
        // FIXME: If the user has set the VARNISH_INCLUDE_PATHS environment variable, use that.
        //    At the moment we have no way to detect which version it is.
        //    vmod_abi.h  seems to have this line, which can be used in the future.
        //    #define VMOD_ABI_Version "Varnish 7.5.0 eef25264e5ca5f96a77129308edb83ccf84cb1b1"
        return Some((
            s.split(':').map(PathBuf::from).collect(),
            "version unknown".into(),
        ));
    }

    let pkg = pkg_config::Config::new();
    match pkg.probe("varnishapi") {
        Ok(l) => {
            let ver = l.version;
            // version string usually looks like "7.5.0"
            let mut parts = ver.split('.');
            let major = parse_next_int(&mut parts, "major");
            let minor = parse_next_int(&mut parts, "minor");
            println!("cargo::metadata=version_number={ver}");
            if major == 7 && minor <= 5 {
                println!("cargo::rustc-cfg=varnishsys_objcore_in_init");
            }
            if major < 6 || major > 7 {
                println!("cargo::warning=Varnish v{ver} is not supported and may not work with this crate");
            }
            Some((l.include_paths, ver))
        }
        Err(e) => {
            // See https://docs.rs/about/builds#detecting-docsrs
            if env::var("DOCS_RS").is_ok() {
                eprintln!("libvarnish not found, using saved bindings for the doc.rs: {e}");
                fs::copy(BINDINGS_FILE, out_path).unwrap();
                println!("cargo::metadata=version_number={BINDINGS_FILE_VER}");
                None
            } else {
                // FIXME: we should give a URL describing how to install varnishapi
                // I tried to find it, but failed to find a clear URL for this.
                panic!("pkg_config failed to find varnishapi, make sure it is installed: {e:?}");
            }
        }
    }
}

fn parse_next_int(parts: &mut Split<char>, name: &str) -> u32 {
    let val = parts
        .next()
        .unwrap_or_else(|| panic!("varnishapi invalid version {name}"));
    val.parse::<u32>()
        .unwrap_or_else(|_| panic!("varnishapi invalid version - {name} value is '{val}'"))
}
