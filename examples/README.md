# Examples

This is a small collection of vmods, written using the [varnish crate](https://crates.io/crates/varnish), each focusing on a different aspect of the API.

- [vmod_example](vmod_example): start with this one for a tour of the different files needed
- [mod_error](mod_error): various ways to convey an error back to VCL when the vmod fails
- [vmod_object](vmod_object): how to map a vmod object into a rust equivalent
- [vmod_timestamp](vmod_timestamp): use of a `PRIV_TASK`
