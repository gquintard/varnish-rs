// Uncomment the following line to disable warnings for the entire crate, e.g. during debugging.
// #![allow(warnings)]

use errors::Errors;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, ItemMod, Type};
use {proc_macro as pm, proc_macro2 as pm2};

use crate::gen_docs::generate_docs;
use crate::generator::render_model;
use crate::parser::tokens_to_model;

mod errors;
mod gen_docs;
mod gen_func;
mod gen_objects;
mod generator;
mod model;
mod names;
mod parser;
mod parser_args;
mod parser_utils;

pub(crate) type ProcResult<T> = Result<T, Errors>;

/// All tests for the proc-macro crate must be part of the crate itself
/// because the tests must call functions not tagged with the `#[proc_macro_attribute]`,
/// but the current proc-macro limitation does not allow these functions to be exported.
/// The only real shortcoming of this approach is that we must add each test file to `tests/mod.rs`
#[cfg(test)]
mod tests;

/// Handle the `#[vmod]` attribute.  This attribute can only be applied to a module.
/// Inside the module, it handles the following items:
/// - Public functions are exported as VMOD functions.
///   - `#[event]` attribute on a function will export it as an event function.
///   - `#[shared_per_task]` attribute on a function argument will treat it as a `PRIV_TASK` object.
///   - `#[shared_per_vcl]` attribute on a function argument will treat it as a `PRIV_VCL` object.
/// - `impl` blocks' public methods are exported as VMOD object methods. The object itself may reside outside the module.
///   - `pub fn new(...)` is treated as the object constructor.
///   - `#[vcl_name]` attribute on an object constructor's argument will set it to the VCL name.
#[proc_macro_attribute]
pub fn vmod(args: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    // parse the module code into a data model.
    // Most error checking is done here.
    // Magical attributes like `#[event]` are removed from the user's code.
    // let args = parse_macro_input!(args);
    // let args = parse_macro_input!(args);
    // let input = parse_macro_input!(input);
    let args = pm2::TokenStream::from(args);
    let mut item_mod = parse_macro_input!(input as ItemMod);

    let info = match tokens_to_model(args, &mut item_mod) {
        Ok(v) => v,
        Err(err) => return err.into_compile_error().into(),
    };

    // generate the code for the VMOD.
    // This will output the slightly modified original user code,
    // plus generate the FFI code as a submodule.
    let result = render_model(item_mod, &info);

    // generate documentation file if needed
    generate_docs(&info);

    result.into()
}

/// Handle the `#[stats]` attribute.  This attribute can only be applied to a struct.
/// The struct must have only fields of type `AtomicU64`.
/// - `#[counter]` attribute on a field will export it as a counter.
/// - `#[gauge]` attribute on a field will export it as a gauge.
#[proc_macro_attribute]
pub fn stats(_args: pm::TokenStream, input: pm::TokenStream) -> pm::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let vis = &input.vis;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    };

    // Ensure all fields are AtomicU64, for now
    for field in fields {
        match &field.ty {
            Type::Path(path) => {
                let is_atomic_u64 = path
                    .path
                    .segments
                    .last()
                    .is_some_and(|seg| seg.ident == "AtomicU64");

                if !is_atomic_u64 {
                    let field_name = field.ident.as_ref().unwrap();
                    panic!("Field {field_name} must be of type AtomicU64");
                }
            }
            _ => panic!("Field types must be AtomicU64"),
        }
    }

    let original_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let vis = &f.vis;
        quote! { #vis #name: std::sync::atomic::AtomicU64 }
    });

    let metrics = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();

        // Look for either counter or gauge attribute
        let (counter_type, attrs) = if let Some(attrs) = field.attrs.iter().find(|attr| attr.path().is_ident("counter")) {
            ("counter", attrs)
        } else if let Some(attrs) = field.attrs.iter().find(|attr| attr.path().is_ident("gauge")) {
            ("gauge", attrs)
        } else {
            panic!("Field {field_name} must have either #[counter] or #[gauge] attribute")
        };

        let mut oneliner = String::new();
        let mut level = String::from("info");
        let mut format = String::from("integer");
        let mut docs = String::new();

        attrs.parse_nested_meta(|meta| {
            if meta.path.is_ident("oneliner") {
                oneliner = meta.value()?.parse::<syn::LitStr>()?.value();
            } else if meta.path.is_ident("level") {
                level = meta.value()?.parse::<syn::LitStr>()?.value();
            } else if meta.path.is_ident("format") {
                format = meta.value()?.parse::<syn::LitStr>()?.value();
                match format.as_str() {
                    "integer" | "bitmap" | "duration" | "bytes" => {},
                    _ => panic!("Invalid format value for field {field_name}. Must be one of: integer, bitmap, duration, bytes")
                }
            } else if meta.path.is_ident("docs") {
                docs = meta.value()?.parse::<syn::LitStr>()?.value();
            }
            Ok(())
        }).unwrap();

        let oneliner = oneliner.as_str();
        let level = level.as_str();
        let format = format.as_str();
        let docs = docs.as_str();

        quote! {
            VscMetricDef {
                name: #field_name,
                counter_type: #counter_type,
                ctype: "uint64_t",
                level: #level,
                oneliner: #oneliner,
                format: #format,
                docs: #docs,
            }
        }
    });

    let name_inner = format_ident!("{}Inner", name);

    quote! {
        use varnish::ffi::vsc_seg;
        use varnish::ffi::{VRT_VSC_Alloc, VRT_VSC_Destroy};
        use varnish::vsc_types::{VscMetricDef, VscCounterStruct};
        use std::ops::{Deref, DerefMut};
        use std::ffi::CString;

        #[repr(C)]
        #[derive(Debug)]
        #vis struct #name_inner {
            #(#original_fields,)*
        }

        // Wrapper struct to hold the VSC segment and the inner counter struct
        #vis struct #name {
            value: *mut #name_inner,
            vsc_seg: *mut vsc_seg,
            name: CString,
        }

        impl Deref for #name {
            type Target = #name_inner;

            fn deref(&self) -> &Self::Target {
                unsafe { &*self.value }
            }
        }

        impl DerefMut for #name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { &mut *self.value }
            }
        }

        impl varnish::vsc_types::VscCounterStruct for #name {
            fn set_vsc_seg(&mut self, seg: *mut vsc_seg) {
                self.vsc_seg = seg;
            }

            // Introspect the struct to get the metric definitions
            fn get_struct_metrics() -> Vec<varnish::vsc_types::VscMetricDef<'static>> {
                vec![
                    #(#metrics),*
                ]
            }

            fn new(module_name: &str, module_prefix: &str) -> #name {
                let mut vsc_seg = std::ptr::null_mut();
                let name = CString::new(module_name).unwrap();
                let format = CString::new(module_prefix).unwrap();

                let json = Self::build_json(module_name);

                let value = unsafe {
                    VRT_VSC_Alloc(
                        std::ptr::null_mut(),
                        &mut vsc_seg,
                        name.as_ptr(),
                        size_of::<#name_inner>(),
                        json.as_ptr(),
                        json.len(),
                        format.as_ptr(),
                        std::ptr::null_mut()
                    ) as *mut #name_inner
                };

                return #name {
                    value,
                    vsc_seg,
                    name,
                }
            }

            fn drop(&mut self) {
                unsafe {
                    VRT_VSC_Destroy(self.name.as_ptr(), self.vsc_seg);
                }
            }
        }
    }
    .into()
}
