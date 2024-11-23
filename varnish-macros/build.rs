fn main() {
    println!("cargo::rustc-check-cfg=cfg(lts_60)");
    if let Ok(v) = std::env::var("DEP_VARNISHAPI_VERSION_NUMBER") {
        if v.starts_with("6.0.") {
            println!("cargo::rustc-cfg=lts_60");
        }
    }
}
