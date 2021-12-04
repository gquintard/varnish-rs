[![crates.io](https://img.shields.io/crates/v/varnish.svg)](https://crates.io/crates/varnish)
[![tests](https://github.com/gquintard/varnish-rs/actions/workflows/tests.yaml/badge.svg)](https://github.com/gquintard/varnish-rs/actions)
[![docs.rs](https://img.shields.io/badge/docs.rs-v0.0.3-brightgreen)](https://docs.rs/varnish/latest/varnish/)

[Documentation](https://docs.rs/varnish/)

In your `Cargo.toml`:

```
[dependencies]
varnish = "0.0.3"
# and if you are building a vmod:
varnish-sys = "0.0.3"
```
# varnish-rs

Varnish bindings, notably to build vmods

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

## Versions

The `varnish-rs` and `varnish-sys` versions will work in tandem: to build version X of `varnish`, you will need `varnish-sys` at the same version X. In turn, `varnish-sys` will depend on a specific Varnish C library version:

| varnish-sys (rust) | libvarnish (C) |
| :----------------: | :------------: |
| 0.0.1              | 7.0            |
| 0.0.2              | 7.0            |
| 0.0.3              | 7.0            |
