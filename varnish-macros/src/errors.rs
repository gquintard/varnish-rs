use std::fmt::Display;

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

use crate::ProcResult;

pub fn error<T: Spanned>(spanned: &T, msg: &str) -> syn::Error {
    syn::Error::new(spanned.span(), msg)
}

pub struct Errors {
    errors: Option<syn::Error>,
}

impl Errors {
    pub fn new() -> Self {
        Self { errors: None }
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_none()
    }

    pub fn on_err<T>(&mut self, result: ProcResult<T>) -> Option<T> {
        match result {
            Ok(val) => Some(val),
            Err(err) => {
                self.combine(err);
                None
            }
        }
    }

    pub fn add<T: Spanned>(&mut self, spanned: &T, msg: &str) {
        let span = spanned.span();
        self.push(syn::Error::new(span, msg));
    }

    pub fn push(&mut self, err: syn::Error) {
        match &mut self.errors {
            Some(errors) => errors.combine(err),
            None => self.errors = Some(err),
        }
    }

    pub fn combine(&mut self, other: Errors) {
        if let Some(errors) = other.errors {
            self.push(errors);
        }
    }

    pub fn into_result(self) -> Result<(), syn::Error> {
        match self.errors {
            Some(errors) => Err(errors),
            None => Ok(()),
        }
    }

    pub fn into_compile_error(self) -> TokenStream {
        match self.errors {
            Some(errors) => errors.to_compile_error(),
            None => quote! {},
        }
    }
}

impl From<syn::Error> for Errors {
    fn from(err: syn::Error) -> Self {
        let mut errors = Self::new();
        errors.push(err);
        errors
    }
}

impl From<darling::Error> for Errors {
    fn from(err: darling::Error) -> Self {
        let mut errors = Self::new();
        errors.push(err.into());
        errors
    }
}

impl Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.errors {
            Some(errors) => write!(f, "{errors}"),
            None => Ok(()),
        }
    }
}
