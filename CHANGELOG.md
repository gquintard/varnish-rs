# 0.0.8 (unreleased)

- switch to a cargo workspace to speed up builds
- fix buffer size issue in VFPs
- do not copy workspace STRINGS into the workspace again
- fix generated code involving default STRING arguments
- C types now derive `Copy`, `Debug` and `Default`
- silence `clippy` for generated code
- expose `http_conn`
- introdude `HTTP::set_status()`, `HTTP::set_prototype()` and `HTTP::set_reason()`

# 0.0.7 (2021-12-26)

- fix a boilerplate issue when using options (extra comma)
- fix handling of option `STRING` parameters
- introduce delivery and fetch processors
- introduce `VPriv::take()`
- introduce `WS::copy_bytes_with_null()`
- `convert` accepts more types, including `Option<&std|String|&[u8]>`
- `example/vmod_vdp`
- `example/vmod_vfp`

# 0.0.6 (2021-12-23)

- `VARNISH_INCLUDE_PATHS` allows to build bindings in non-standard environments
- `vtc!` macro will also print `stderr` in case of failure
- introduce `Ctx::req_body()`

# 0.0.5 (2021-12-18)

- introduce `WS::reserve()` and `WS::release()`
- introduce `Ctx::log`
- `libvarnishapi` version in `varnish-sys` metadata
- more robust `IntoResult` implementation
- All VCL types are recognized (but not necessarily completely "rustified")
- vmod event support
- `example/vmod_event`

# 0.0.4 (2021-12-05)

- documentation starts getting a bit serious
- CI with github actions
- simplify the vmod structure, `src/vmod.rs` disappears
- `example/vmod_timestamp`
- `example/vmod_infiniteloop`

# 0.0.3 (2021-12-01)

- docs can be built even without `libvarnishapi` installed
- fix vmod object support
- `example/vmod_object`

# 0.0.2 (2021-11-30)

- vmods can return a Result that will automatically call VRT_fail() if needed
- `example/vmod_error`
- `example/vmod_example`

# 0.0.1 (2021-11-28)

Initial release
