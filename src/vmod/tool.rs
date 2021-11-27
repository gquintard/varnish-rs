use std::env;
use std::env::join_paths;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn generate() -> Result<(), String> {
    println!("cargo:rerun-if-changed=vmod.vcc");

    let rstool_bytes = include_bytes!("vmodtool-rs.py");
    let rs_tool_path =
        join_paths([env::var("OUT_DIR").unwrap(), String::from("rstool.py")]).unwrap();
    fs::write(&rs_tool_path, &rstool_bytes).expect(&format!(
        "couldn't write rstool.py tool in {:?}",
        &*rs_tool_path
    ));

    let vmodtool_path = pkg_config::get_variable("varnishapi", "vmodtool").unwrap();
    let vmodtool_dir = (vmodtool_path.as_ref() as &Path)
        .parent()
        .expect("couldn't find the directory name containing vmodtool.py")
        .to_str()
        .unwrap()
        .to_string();

    let cmd = Command::new("python3")
        .arg(rs_tool_path)
        .arg("vmod.vcc")
        .arg("-w")
        .arg(env::var("OUT_DIR").unwrap())
        .env(
            "PYTHONPATH",
            join_paths([env::var("OUT_DIR").unwrap_or(String::new()), vmodtool_dir]).unwrap(),
        )
        .output()
        .expect("failed to run vmodtool");

    io::stdout().write_all(&cmd.stderr).unwrap();
    assert!(cmd.status.success());

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("generated.rs");
    fs::write(out_path, &cmd.stdout).expect("Couldn't write boilerplate!");
    Ok(())
}
