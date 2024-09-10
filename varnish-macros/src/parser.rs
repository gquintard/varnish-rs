//! Parser takes in the token stream and generates the model structure.
//! The model describes the functions, objects, and events that should be exposed to Varnish.
//! The model should be thoroughly validated before generating the output, and is treated as the source of truth.

use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro2::TokenStream;
use syn::{Attribute, ImplItem, Item, ItemImpl, ItemMod, Signature, Visibility};

use crate::errors::Errors;
use crate::model::{
    FuncInfo, FuncType, ObjInfo, ParamType, ParamTypeInfo, ReturnTy, ReturnType, SharedTypes,
    VmodInfo, VmodParams,
};
use crate::parser_args::FuncStatus;
use crate::{parser_utils, ProcResult};

pub fn tokens_to_model(args: TokenStream, item_mod: &mut ItemMod) -> ProcResult<VmodInfo> {
    let args = NestedMeta::parse_meta_list(args).map_err(syn::Error::from)?;
    let args = VmodParams::from_list(&args)?;
    let info = VmodInfo::parse(args, item_mod)?;
    Ok(info)
}

impl VmodInfo {
    /// Parse the `mod` item and generate the model of everything
    fn parse(params: VmodParams, item: &mut ItemMod) -> ProcResult<Self> {
        let mut errors = Errors::new();
        let mut funcs = Vec::<FuncInfo>::new();
        let mut objects = Vec::<ObjInfo>::new();
        let mut shared_types = SharedTypes::default();
        let mut has_event = false;
        if let Some((_, content)) = &mut item.content {
            for item in content {
                if let Item::Fn(fn_item) = item {
                    // a function or an event handler
                    let func = FuncInfo::parse(
                        &mut shared_types,
                        &mut fn_item.sig,
                        &fn_item.vis,
                        &mut fn_item.attrs,
                        false,
                    );
                    if let Some(func) = errors.on_err(func) {
                        if matches!(func.func_type, FuncType::Event) {
                            if has_event {
                                errors.add(&fn_item.sig.ident, "Only one event handler is allowed");
                                continue;
                            }
                            has_event = true;
                        }
                        funcs.push(func);
                    }
                } else if let Item::Impl(impl_item) = item {
                    // an object
                    if let Some(obj) = errors.on_err(ObjInfo::parse(impl_item, &mut shared_types)) {
                        objects.push(obj);
                    }
                }
            }
        }
        if funcs.is_empty() && objects.is_empty() {
            errors.add(&item.ident, "No functions or objects found in this module");
        }

        errors.into_result()?;
        Ok(Self {
            params,
            ident: item.ident.to_string(),
            docs: parser_utils::parse_doc_str(&item.attrs),
            shared_types,
            funcs,
            objects,
        })
    }
}

impl ObjInfo {
    /// Parse an `impl` block and treat all public functions as object methods
    fn parse(item_impl: &mut ItemImpl, shared_types: &mut SharedTypes) -> ProcResult<Self> {
        let mut errors = Errors::new();
        let ident = parser_utils::as_simple_ty(item_impl.self_ty.as_ref()).map(|v| v.to_string());

        // Add only one error per object impl declaration
        if item_impl.trait_.as_ref().is_some() {
            errors.add(&item_impl, "Trait impls are not supported for object impls");
        } else if !item_impl.generics.params.is_empty() {
            errors.add(
                &item_impl.generics.params,
                "Generics are not supported for object impls",
            );
        } else if ident.is_none() {
            errors.add(
                &item_impl.self_ty,
                "Expected a simple type for object. If the object is defined elsewhere, use `use` to import it.",
            );
        }

        let mut funcs = Vec::new();
        let mut constructor = None;
        for item in &mut item_impl.items {
            if let ImplItem::Fn(fn_item) = item {
                let Some(func) = errors.on_err(FuncInfo::parse(
                    shared_types,
                    &mut fn_item.sig,
                    &fn_item.vis,
                    &mut fn_item.attrs,
                    true,
                )) else {
                    continue;
                };
                if func.ident == "new" {
                    constructor = Some(func);
                } else {
                    funcs.push(func);
                }
            }
        }

        if constructor.is_none() {
            errors.add(
                &item_impl.self_ty,
                "Object must have a constructor called `new`",
            );
        }

        errors.into_result()?;
        Ok(Self {
            ident: ident.expect("ident err already reported"),
            docs: parser_utils::parse_doc_str(&item_impl.attrs),
            constructor: constructor.expect("ctor err already reported"),
            destructor: FuncInfo {
                func_type: FuncType::Destructor,
                ident: "_fini".to_string(),
                docs: String::new(),
                has_optional_args: false,
                args: Vec::new(),
                returns: ReturnType::Value(ReturnTy::Default),
            },
            funcs,
        })
    }
}

impl FuncInfo {
    /// Parse a function or a method signature
    fn parse(
        shared_types: &mut SharedTypes,
        signature: &mut Signature,
        vis: &Visibility,
        attrs: &mut Vec<Attribute>,
        is_object: bool,
    ) -> ProcResult<Self> {
        let mut errors = Errors::new();

        if !matches!(vis, Visibility::Public(..)) {
            errors.add(
                signature, // cannot use `vis` because it might be `Inherited`
                "Only public functions and impl blocks are allowed inside a `mod` tagged with `#[varnish::vmod]`. Add `pub` or move this function outside of this mod.",
            );
        } else if signature.asyncness.is_some() {
            errors.add(signature, "async functions are not supported");
        } else if signature.unsafety.is_some() {
            errors.add(signature, "unsafe functions are not supported");
        }

        let func_type = if let Some(attr) = parser_utils::remove_attr(attrs, "event") {
            if is_object {
                errors.add(
                    &attr.meta,
                    "Event functions are not supported for object methods",
                );
            }
            FuncType::Event
        } else if is_object {
            if signature.ident == "new" {
                FuncType::Constructor
            } else {
                FuncType::Method
            }
        } else {
            FuncType::Function
        };

        let returns = errors.on_err(ReturnType::parse(&signature.output, func_type));
        let mut status = FuncStatus::new(func_type);
        let mut args = Vec::new();

        for (idx, arg) in signature.inputs.iter_mut().enumerate() {
            let arg = ParamTypeInfo::parse(shared_types, &mut status, idx, arg);
            if let Some(arg) = errors.on_err(arg) {
                args.push(arg);
            }
        }

        let has_optional_args = args
            .iter()
            .any(|arg| matches!(&arg.ty, ParamType::Value(v) if v.is_optional));

        errors.into_result()?;
        Ok(Self {
            func_type,
            ident: signature.ident.to_string(),
            docs: parser_utils::parse_doc_str(attrs),
            has_optional_args,
            returns: returns.expect("returns err already reported"),
            args,
        })
    }
}
