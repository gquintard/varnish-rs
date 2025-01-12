use quote::quote;
use syn::{Data, Fields, Type};

pub fn get_struct_fields(
    data: &Data,
) -> &syn::punctuated::Punctuated<syn::Field, syn::token::Comma> {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Only named fields are supported"),
        },
        _ => panic!("Only structs are supported"),
    }
}

pub fn validate_fields(fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) {
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
}

pub fn generate_field_definitions(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    fields.iter().map(|f| {
        let name = &f.ident;
        let vis = &f.vis;
        quote! { #vis #name: std::sync::atomic::AtomicU64 }
    })
}

pub fn parse_doc_comments(field: &syn::Field) -> (String, String) {
    let mut doc_lines = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            let syn::Meta::NameValue(meta) = &attr.meta else {
                return None;
            };
            let syn::Expr::Lit(expr) = &meta.value else {
                return None;
            };
            let syn::Lit::Str(lit) = &expr.lit else {
                return None;
            };
            Some(lit.value())
        })
        .filter(|s| !s.is_empty());

    let oneliner = doc_lines.next().unwrap_or_default();
    let docs = doc_lines.next().unwrap_or_default();
    (oneliner, docs)
}

pub fn parse_counter_attributes(field: &syn::Field, counter_type: &str) -> (String, String) {
    let mut level = String::from("info");
    let mut format = String::from("integer");

    if let Some(attrs) = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident(counter_type))
    {
        let _ = attrs.parse_nested_meta(|meta| {
          match meta.path.get_ident().map(ToString::to_string).as_deref() {
              Some("level") => {
                  level = meta.value()?.parse::<syn::LitStr>()?.value();
              }
              Some("format") => {
                  format = meta.value()?.parse::<syn::LitStr>()?.value();
                  let field_name = field.ident.as_ref().unwrap();
                  assert!(
                      ["integer", "bitmap", "duration", "bytes"].contains(&format.as_str()),
                      "Invalid format value for field {field_name}. Must be one of: integer, bitmap, duration, bytes"
                  );
              }
              _ => {}
          }
          Ok(())
      });
    }
    (level, format)
}

pub fn generate_metrics(
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> impl Iterator<Item = proc_macro2::TokenStream> + '_ {
    fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();

        let counter_type = if field
            .attrs
            .iter()
            .any(|attr| attr.path().is_ident("counter"))
        {
            "counter"
        } else if field.attrs.iter().any(|attr| attr.path().is_ident("gauge")) {
            "gauge"
        } else {
            panic!("Field {field_name} must have either #[counter] or #[gauge] attribute")
        };

        let (oneliner, docs) = parse_doc_comments(field);
        let (level, format) = parse_counter_attributes(field, counter_type);

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
    })
}

pub fn generate_output(
    name: &syn::Ident,
    name_inner: &proc_macro2::Ident,
    vis: &syn::Visibility,
    original_fields: impl Iterator<Item = proc_macro2::TokenStream>,
    metrics: impl Iterator<Item = proc_macro2::TokenStream>,
) -> proc_macro2::TokenStream {
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
}
