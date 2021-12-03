# vmod_example

Here you will find a starting point for your own vmods, and to learn coding vmods in `rust`. Ideally, you should be familiar at least with either vmod development, or with `rust` development, but if your starting fresh, this should get you going too.

# Compiling

You need only two things:
- a stable version of `cargo`/`rust`
- the `libvarnish` development files installed where `pkg-config` can find them

From within this directory, run:

```
# build
cargo build
# you should now have a file name target/debug/libvmod_example.so

# test (you need to build first!)
cargo test
```

That's it!

# Files

Look around, everything should be decently documented:
- [vmod.vcc](vmod.vcc): your starting point, where you will describe your vmod API
- [src/vmod.rs](src/vmod.rs): the file contianing the actual implementation and unit tests
- [tests/test01.vtc](tests/test01.vtc): a VTC (full stack) test, actually running Varnish against mock clients and servers
- [Cargo.toml](Cargo.toml): the file describing the name of the vmod, as well as its dependencies
- [src/lib.rs](src/lib.rs): this is the entry point for the compiler, not much to see
- [build.rs](build.rs): a short program in charge of generating some boilerplate before the compiler starts
