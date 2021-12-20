# 0.0.6 (unreleased)

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

# 0.0.5 (2021-12-05)

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
