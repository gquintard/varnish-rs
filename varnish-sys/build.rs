use convert_case::Case::Pascal;
use convert_case::Casing;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, Default)]
struct ParseCallbacks {
    /// C items and their Rust names
    item_names: HashMap<&'static str, &'static str>,
    /// C enums (i.e. "enum foo"),  the prefix to remove from their values,
    /// and explicit renames for some values without prefix
    enum_renames: HashMap<&'static str, (&'static str, HashMap<&'static str, &'static str>)>,
}

impl ParseCallbacks {
    fn get_regex_str(&self) -> String {
        self.item_names.keys().fold(String::new(), |mut acc, x| {
            if !acc.is_empty() {
                acc.push('|');
            }
            acc.push_str(x);
            acc
        })
    }
}

impl bindgen::callbacks::ParseCallbacks for ParseCallbacks {
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        value: &str,
        _variant_value: bindgen::callbacks::EnumVariantValue,
    ) -> Option<String> {
        self.enum_renames.get(enum_name?).map(|v| {
            let val = value.trim_start_matches(v.0);
            v.1.get(val)
                .map(|x| (*x).to_string())
                .unwrap_or(val.to_case(Pascal))
        })
        // // Print unrecognized enum values for debugging
        // .or_else(|| {
        //     let name = enum_name.unwrap();
        //     println!("cargo::warning=Unrecognized {name} - {value}");
        //     None
        // })
    }

    fn item_name(&self, item_name: &str) -> Option<String> {
        self.item_names.get(item_name).map(|x| (*x).to_string())
    }
}

macro_rules! rename {
    ( $cb:expr, $name_c:literal, $name_rs:literal, $rm_prefix:literal $(, $itm:literal => $ren:literal)* $(,)? ) => {
        $cb.item_names.insert($name_c, $name_rs);
        $cb.enum_renames.insert(concat!("enum ", $name_c), (
            $rm_prefix,
            vec![$( ($itm, $ren), )*].into_iter().collect()));
    };
}

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
                    println!("cargo::rustc-cfg=feature=\"_objcore_in_init\"");
                }
                l.include_paths
            }
            Err(e) => {
                // See https://docs.rs/about/builds#detecting-docsrs
                if env::var("DOCS_RS").is_ok() {
                    eprintln!("libvarnish not found, using saved bindings for the doc.rs: {e}");
                    fs::copy("bindings.for-docs", out_path).unwrap();
                    println!("cargo::metadata=version_number=7.6.0");
                    return;
                }
                // FIXME: we should give a URL describing how to install varnishapi
                // I tried to find it, but failed to find a clear URL for this.
                panic!("pkg_config failed to find varnishapi, make sure it is installed: {e:?}");
            }
        }
    };

    let mut cb = ParseCallbacks::default();
    rename!(cb, "VSL_tag_e", "VslTag", "SLT_"); // SLT_Debug
    rename!(cb, "boc_state_e", "BocState", "BOS_"); // BOS_INVALID
    rename!(cb, "director_state_e", "DirectorState", "DIR_S_", "HDRS" => "Headers"); // DIR_S_NULL
    rename!(cb, "gethdr_e", "GetHeader", "HDR_"); // HDR_REQ_TOP
    rename!(cb, "sess_attr", "SessionAttr", "SA_"); // SA_TRANSPORT
    rename!(cb, "lbody_e", "Body", "LBODY_"); // LBODY_SET_STRING
    rename!(cb, "task_prio", "TaskPriority", "TASK_QUEUE_"); // TASK_QUEUE_BO
    rename!(cb, "vas_e", "Vas", "VAS_"); // VAS_WRONG
    rename!(cb, "vcl_event_e", "VclEvent", "VCL_EVENT_"); // VCL_EVENT_LOAD
    rename!(cb, "vcl_func_call_e", "VclFuncCall", "VSUB_"); // VSUB_STATIC
    rename!(cb, "vcl_func_fail_e", "VclFuncFail", "VSUB_E_"); // VSUB_E_OK
    rename!(cb, "vdp_action", "VdpAction", "VDP_"); // VDP_NULL
    rename!(cb, "vfp_status", "VfpStatus", "VFP_"); // VFP_ERROR

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
        .new_type_alias("VCL_BLOB") // *const vrt_blob
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
        // FIXME: some enums should probably be done as rustified_enum (exhaustive)
        .rustified_non_exhaustive_enum(cb.get_regex_str())
        .parse_callbacks(Box::new(cb))
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
