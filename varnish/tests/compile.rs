// Use this hack to include a specific file into the regular IDE code parsing to help fix a specific test file
// #[path = "pass/vcl_returns.rs"]
// mod try_to_build;

#[cfg(varnishsys_6)]
static EXCLUDE_FILES_V6: &[&str] = &[
    "pass/event3.rs",
    "pass/event4.rs",
    "pass/function.rs",
    "pass_ffi/vcl_returns.rs",
];

#[test]
fn compile_expected_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
}

#[test]
fn compile_valid() {
    compile_pass("tests/pass/*.rs");
}

#[cfg(feature = "ffi")]
#[test]
fn compile_valid_ffi_code() {
    compile_pass("tests/pass_ffi/*.rs");
}

fn compile_pass(pattern: &str) {
    let t = trybuild::TestCases::new();
    for file in glob::glob(pattern).unwrap() {
        let file = file.unwrap();
        #[cfg(varnishsys_6)]
        {
            let filepath = file.to_str().unwrap();
            if EXCLUDE_FILES_V6.iter().any(|&f| filepath.ends_with(f)) {
                eprintln!("Skipping file: {filepath} because of the old varnish version");
                continue;
            }
        }
        t.pass(file);
    }
}
