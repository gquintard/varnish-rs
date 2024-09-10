# Tests

There are two types of tests here.  To run all tests, use `cargo test -p varnish-macros`.

### `trybuild` tests
[trybuild](https://docs.rs/trybuild/latest/trybuild/) tests, which verify that the macro code can be compiled, or when there is a problem, the compiler errors are properly reported. The test cases are in the `tests/pass/*.rs` and `tests/fail/*.rs`, and the expected errors are in the `tests/fail/*.stderr` files. To update the expected output, make sure `TRYBUILD=overwrite` is set, i.e. just run `TRYBUILD=overwrite cargo test -p varnish-macros`.

### `insta` tests
[insta](https://insta.rs/docs/cli/) test, which verifies that the macros produce the correct output when used correctly, and also that the output is stable. The test uses the same `tests/pass/*.rs` files, and the expected output is in the `src/tests/snapshots/*.snap` files. The tests must be part of the crate itself due to it being proc-macro. To update the snapshots, run `cargo insta test -p varnish-macros --accept`, which requires insta to be installed.

Both test results can be run together: `TRYBUILD=overwrite cargo insta test -p varnish-macros --accept`
