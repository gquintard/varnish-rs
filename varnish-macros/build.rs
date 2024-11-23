fn main() {
    println!("cargo::rustc-check-cfg=cfg(lts_60)");
    println!("cargo::rustc-check-cfg=cfg(varnishsys_use_priv_free_f)");
    if let Ok(v) = std::env::var("DEP_VARNISHAPI_VERSION_NUMBER") {
        if v.starts_with("6.0.") {
            println!("cargo::rustc-cfg=lts_60");
            println!("cargo::rustc-cfg=varnishsys_use_priv_free_f");
        }
    }
}
