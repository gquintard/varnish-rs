//! A module to generate names for the generated code.

use std::ffi::CString;
use std::fmt::Display;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, IdentFragment};

use crate::model::FuncType;

/// A struct to generate all types of names for the generated code.
/// An instance of this struct is held by each type of items - modules, functions/methods, and objects.
#[derive(Debug, Default)]
pub struct Names {
    /// Name of the user's `mod`
    mod_name: String,
    obj_name: Option<String>,
    fn_name: Option<(FuncType, String)>,
}

impl Names {
    pub fn new(mod_name: &str) -> Self {
        Self {
            mod_name: mod_name.to_string(),
            obj_name: None,
            fn_name: None,
        }
    }

    pub fn to_obj(&self, obj_name: &str) -> Self {
        assert!(self.obj_name.is_none());
        assert!(self.fn_name.is_none());
        Self {
            mod_name: self.mod_name.clone(),
            obj_name: Some(obj_name.to_string()),
            fn_name: None,
        }
    }

    pub fn to_func(&self, func_type: FuncType, fn_name: &str) -> Self {
        assert!(self.fn_name.is_none());
        Self {
            mod_name: self.mod_name.clone(),
            obj_name: self.obj_name.clone(),
            fn_name: Some((func_type, fn_name.to_string())),
        }
    }

    pub fn mod_name(&self) -> &str {
        &self.mod_name
    }

    pub fn obj_name(&self) -> &str {
        self.obj_name.as_ref().unwrap()
    }

    pub fn fn_name(&self) -> &str {
        let (ty, name) = self.fn_name.as_ref().unwrap();
        match *ty {
            FuncType::Constructor => "_init",
            FuncType::Destructor => "_fini",
            _ => name.as_str(),
        }
    }

    pub fn fn_name_user(&self) -> &str {
        self.fn_name.as_ref().unwrap().1.as_str()
    }

    pub fn fn_callable_name(&self, func: FuncType) -> TokenStream {
        let name = self.fn_name_user().to_ident();
        match func {
            FuncType::Constructor => {
                let obj = self.obj_access();
                quote! { #obj::#name }
            }
            FuncType::Method => quote! { __obj.#name },
            _ => quote! { super::#name },
        }
    }

    pub fn obj_access(&self) -> TokenStream {
        let name = self.obj_name().to_ident();
        quote! { super::#name }
    }

    pub fn func_struct_name(&self) -> String {
        format!("Vmod_vmod_{}_Func", self.mod_name)
    }

    pub fn data_struct_name(&self) -> String {
        format!("Vmod_{}_Data", self.mod_name)
    }

    pub fn struct_obj_name(&self) -> String {
        format!("struct vmod_{}_{}", self.mod_name, self.obj_name())
    }

    pub fn wrapper_fn_name(&self) -> String {
        let (underscore, obj_name) = self.obj_name_parts();
        format!("vmod_c{underscore}{obj_name}_{}", self.fn_name())
        // format!("vmod_wrapper{underscore}{obj_name}_{}", self.fn_name())
    }

    pub fn arg_struct_name(&self) -> String {
        let (underscore, obj_name) = self.obj_name_parts();
        format!(
            "arg_vmod_{}{underscore}{obj_name}_{}",
            self.mod_name,
            self.fn_name()
        )
    }

    pub fn typedef_name(&self) -> String {
        let (underscore, obj_name) = self.obj_name_parts();
        format!(
            "td_vmod_{}{underscore}{obj_name}_{}",
            self.mod_name,
            self.fn_name()
        )
    }

    pub fn f_fn_name(&self) -> String {
        let (underscore, obj_name) = self.obj_name_parts();
        format!("f{underscore}{obj_name}_{}", self.fn_name())
    }

    // Helper utils

    fn obj_name_parts(&self) -> (&str, &str) {
        let underscore = self.obj_name.as_ref().map_or("", |_| "_");
        let obj_name = self.obj_name.as_ref().map_or("", |v| v.as_str());
        (underscore, obj_name)
    }
}

pub trait ForceCstr {
    fn force_cstr(&self) -> CString;
}

impl ForceCstr for String {
    fn force_cstr(&self) -> CString {
        CString::new(self.as_str()).unwrap()
    }
}

impl ForceCstr for str {
    fn force_cstr(&self) -> CString {
        CString::new(self).unwrap()
    }
}

pub trait ToIdent {
    fn to_ident(&self) -> Ident;
}

impl<T: Display + IdentFragment> ToIdent for T {
    fn to_ident(&self) -> Ident {
        format_ident!("{self}")
    }
}
