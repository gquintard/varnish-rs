// Use this hack to include a specific file into the regular IDE code parsing to help fix a specific test file
// #[path = "pass/function.rs"]
// mod try_to_build;

#[test]
fn compile_expected_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/fail/*.rs");
}

#[test]
fn compile_valid_code() {
    let t = trybuild::TestCases::new();
    t.pass("tests/pass/*.rs");
}
