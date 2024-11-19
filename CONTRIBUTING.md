# Testing

There are two types of tests in this project:

### `trybuild` tests
[trybuild](https://docs.rs/trybuild/latest/trybuild/) tests, which verify that the macro code can be compiled, or when there is a problem, the compiler errors are properly reported. The test cases are in the `varnish/tests` -- `pass/*.rs` work by default, `pass-ffi/*.rs` work only with the `ffi` feature is enabled, and `fail/*.rs` should fail. The expected errors are in the `fail/*.stderr` files. To update the expected output, run `just bless`.

### `insta` tests
[insta](https://insta.rs/docs/cli/) tests, which verify that the macros produce the correct output when used correctly, and also that the output is stable. The test uses the same `pass` and `pass-ffi` files, and the expected output is in the `varnish/snapshots/*.snap` files. The test runner must be part of the crate itself due to it being proc-macro, so it is located in `varnish-macros/src/tests.rs`. To update the snapshots, use the same `just bless` command. Make sure to install insta first.
