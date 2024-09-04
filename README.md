[![GitHub](https://img.shields.io/badge/github-varnish-8da0cb?logo=github)](https://github.com/gquintard/varnish-rs)
[![crates.io version](https://img.shields.io/crates/v/varnish.svg)](https://crates.io/crates/varnish)
[![docs.rs docs](https://docs.rs/varnish/badge.svg)](https://docs.rs/varnish)
[![crates.io version](https://img.shields.io/crates/l/varnish.svg)](https://github.com/gquintard/varnish-rs/blob/main/LICENSE)
[![CI build](https://github.com/gquintard/varnish-rs/actions/workflows/tests.yaml/badge.svg)](https://github.com/gquintard/varnish-rs/actions)

[Documentation](https://docs.rs/varnish/)

In your `Cargo.toml`:

```
[dependencies]
varnish = "0.0.16"
```

# varnish-rs

Varnish bindings, notably to build vmods, such as:

- [vmod-reqwest](https://github.com/gquintard/vmod_reqwest): issue HTTP calls from VCL, use dynamic, HTTPS backends (support HTTP2)
- [vmod-rers](https://github.com/gquintard/vmod_rers): support for dynamic regex, including respone body manipulation
- [vmod-fileserver](https://github.com/gquintard/vmod_fileserver): serve files directly from disk, without the need for an HTTP backend

Don't hesitate to open github issues if something is unclear or impractical. You can also join us on [discord](https://discord.com/invite/EuwdvbZR6d).

## Requirements

### Rust

`varnish-rs` works on stable Rust and should be fine with more recent versions too. If it doesn't, please open an issue.

### Varnish

`varnish-rs` relies on `varnish-sys` (in this same repository) to generate bindings from the `libvarnish` headers which you will need to install, depending on you linux distribution, the related package can be named `varnish-devel`, `varnish-dev` or maybe `libvarnish-dev`.

Right now, the only Varnish versions supported are `7.*`.

## Python3

At the moment, we use an embedded [python script](src/vmodtool-rs.py) to generate the boilerplate that exposes the rust vmod code to Varnish. Make sure that `python3` is in your path, or that the `PYTHON` environment variable is pointing at a compatible interpreter.

## Building

``` bash
git clone https://github.com/gquintard/varnish-rs.git
cd varnish-rs
cargo build
```

If your `varnish` headers are installed where `pkg-config` can find them, it's all there is to it. If not, you can set the `VARNISH_INCLUDE_PATHS` environment variable to a colon-separated list of paths to search:

```
VARNISH_INCLUDE_PATHS=/my/custom/libpath:/my/other/custom/libpath cargo build
```

## Versions

The `varnish-rs` and `varnish-sys` versions will work in tandem: to build version X of `varnish`, you need version X of `varnish-rs` and of `varnish-sys`, in turn `varnish-sys` will depend on a specific Varnish C library version:

| varnish-rs/varnish-sys (rust) | libvarnish (C) |
| :----------------: | :------------: |
| 0.0.18 -> 0.0.19   | 7.5            |
| 0.0.17             | 7.4            |
| 0.0.15 -> 0.0.16   | 7.3            |
| 0.0.12 -> 0.0.14   | 7.2            |
| 0.0.9 -> 0.0.11    | 7.1            |
| 0.0.*              | 7.0            |

You can check which Varnish version is required using the `libvarnish` metadata field of `varnish-sys`:

``` bash
cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "varnish-sys") | .metadata.libvarnishapi.version '
```
