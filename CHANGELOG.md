# 0.0.15 (unreleased)

- Varnish 7.3 support
- Fix crash in `Backend` code due do wrong pointer cast
- Add `varnish::vsc`
- More docs

# 0.0.14 (2023-02-20)

- `Backend::new()` get an extra argument: `has_probe`
- `Probe` is renamed `COWProbe`, and `Probe` now owns its `String`s

# 0.0.13 (2023-02-12)

- `cache_director.h` added to `varnish-sys`
- `generate_boilerplate!` now reads the `PYTHON` environment variable before falling
  back to `python3` to generate code
- `varnish-rs` reexports the VCL types needed to generate boilerplate code, so that
  vmods don't need to add `varnish-sys` to their `Cargo.toml` anymore.
- introduce `vcl::Backend` and co.
- `VCL_IP can be translated to `Option<std::net::SockAdd>` and back
- fix compilation issues on arm
- introduce `vcl::Error` and `vcl::Result`

# 0.0.12 (2022-11-23)

- ctx->handling is now hidden from us, so we use `VRT_fail`, at the cost of a string copy
- the JSON format in vmod_data changed slightly, adjust for that
- explicitly `drop` `Box::from_raw` results to silence `rustc`

# 0.0.11 (2022-06-16)

- fix generated `__fini` prototype

# 0.0.10 (2022-06-16)

- dumb vsb support (`vcl::vsb::Vsb`))
- probe support (`vcl::probe::Probe`)
- vmod object constructors must now return results
- `Intoresult` require the `Err()` to implement `ToString()`

# 0.0.9 (2022-05-01)

- adjust to Varnish 7.1.0

# 0.0.8 (2022-01-30)

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
