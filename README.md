# varnish-rs

Varnish bindings, notably to build vmods

## Requirements

### Rust

`varnish-rs` works on stable Rust and should be fine with more recent versions too. If it doesn't, plus open an issue.

### Varnish

`varnish-rs` relies on `varnish-sys` (in this same repository) to generate bindings from the `libvarnish` headers which you will need to install, depending on you linux distribution, the related package can be named `varnish-devel`, `varnish-dev` or maybe `libvarnish-dev`.

Right now, the only Varnish version supported is `7.0`.

## Building

``` bash
git clone https://github.com/gquintard/varnish-rs.git
cd varnish-rs
cargo build
```
