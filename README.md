[![crates.io](https://img.shields.io/crates/v/varnish.svg)](https://crates.io/crates/varnish)
[![tests](https://github.com/gquintard/varnish-rs/actions/workflows/tests.yaml/badge.svg)](https://github.com/gquintard/varnish-rs/actions)
[![docs.rs](https://img.shields.io/badge/docs.rs-v0.0.8-brightgreen)](https://docs.rs/varnish/latest/varnish/)

[Documentation](https://docs.rs/varnish/)

In your `Cargo.toml`:

```
[dependencies]
varnish = "0.0.8"
# and if you are building a vmod:
varnish-sys = "0.0.8"
```
# varnish-rs

Varnish bindings, notably to build vmods

Don't hesitate to open github issues if something is unclear or impractical. You can also join us on [discord](https://discord.com/invite/EuwdvbZR6d).

## Requirements

### Rust

`varnish-rs` works on stable Rust and should be fine with more recent versions too. If it doesn't, plus open an issue.

### Varnish

`varnish-rs` relies on `varnish-sys` (in this same repository) to generate bindings from the `libvarnish` headers which you will need to install, depending on you linux distribution, the related package can be named `varnish-devel`, `varnish-dev` or maybe `libvarnish-dev`.

Right now, the only Varnish version supported is `7.0`.

## Python3

At the moment, we use an embedded [python script](src/vmodtool-rs.py) to generate the boilerplate that exposes the rust vmod code to Varnish. Make sure that `python3` is in your path.

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

The `varnish-rs` and `varnish-sys` versions will work in tandem: to build version X of `varnish`, you need version X of `varnish-sys`, in turn `varnish-sys` will depend on a specific Varnish C library version:

| varnish-sys (rust) | libvarnish (C) |
| :----------------: | :------------: |
| 0.0.9 -> 0.0.11    | 7.1            |
| 0.0.*              | 7.0            |

You can check which Varnish version is required using the `libvarnish` metadata field of `varnish-sys`:

``` bash
cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "varnish-sys") | .metadata.libvarnishapi.version '
```
