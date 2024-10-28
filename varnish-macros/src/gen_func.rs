//! Generates functions, methods, and events code for the Varnish VMOD.

use std::fmt::Write as _;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde_json::{json, Value};

use crate::model::FuncType::{Constructor, Destructor, Event, Function, Method};
use crate::model::{FuncInfo, ParamTy, ParamType, ParamTypeInfo, ReturnTy, ReturnType};
use crate::names::{Names, ToIdent};

#[derive(Debug, Default)]
pub struct FuncProcessor {
    names: Names,

    /// For fn with optional args, the name of the struct that holds all arguments, i.e. `arg_simple_void_to_void`
    opt_args_ty_name: String,

    /// Rust wrapper steps before calling user fn, e.g. create temp vars from C values. `[ {let ctx = &mut ctx; }, {let __var1 = args.foo;} ]`
    func_pre_call: Vec<TokenStream>,
    /// Arguments as passed to the user function from the Rust wrapper: `[ {&ctx}, {__var1;} ]`
    func_call_vars: Vec<TokenStream>,
    /// Rust wrapper steps to take after calling user function, e.g. `[ { if ... { ... } } ]`
    func_post_call: Vec<TokenStream>,

    /// C function list of arguments for funcs with no optional args, e.g. `["VCL_INT", "VCL_STRING"]`
    cproto_wrapper_args: Vec<&'static str>,
    /// For optional arguments, params to go into the C header, i.e. `[ {c_char valid_arg0}, {VCL_INT arg1} ]`
    cproto_opt_arg_decl: Vec<String>,
    /// For optional arguments, params to go into `args` struct, i.e. `[ {valid_arg0: c_char}, {arg1: VCL_INT} ]`
    opt_args_arg_decl: Vec<TokenStream>,
    /// Args to the export "C" function. Similar to `opt_args_arg_decl`, i.e. `[ {ctx: *mut vrt_ctx}, {arg0: VCL_INT} ]`
    wrap_fn_arg_decl: Vec<TokenStream>,
    /// Corresponding strings to be added to C declaration, matching `wrap_fn_arg_decl`
    cproto_fn_arg_decl: Vec<String>,

    /// List of arguments as published in the JSON, with up to five values each e.g. `[[INT, ...], [STRING]]`
    /// Order:  `[VCC_type, arg_name, default_value, spec(?), is_optional]`
    /// Any arg after the first one is optional - `NULL`, and all trailing `NULLs` should be removed.
    args_json: Vec<Value>,

    /// `-> c_output_type` or empty if the export "C" function returns nothing
    wrap_fn_output: TokenStream,
    /// VCL types as used in the .c and .h files, e.g. `VCL_INT`, `VCL_STRING`, `VCL_VOID`, ...
    output_hdr: String,
    /// VCC types as used in the .vcc file, e.g. `INT`, `STRING`, `VOID`, ...
    output_vcc: String,

    /// `typedef VCL_VOID td_simple_void_to_void(VRT_CTX, VCL_STRING, ...);` C code
    pub cproto_typedef_decl: String,
    /// `td_simple_void_to_void *f_void_to_void;` - part of the `struct Vmod_something { ... }` C code
    pub cproto_typedef_init: String,

    /// `rust_fn_name: Option< unsafe extern "C" fn name_c_fn(c_args) -> c_output_type >`
    pub export_decl: TokenStream,
    /// `rust_fn_name: Some(name_c_fn)`
    pub export_init: TokenStream,
    /// Full body of the export "C" function
    pub wrapper_function_body: TokenStream,
    /// JSON blob for the function
    pub json: Value,
}

impl FuncProcessor {
    pub fn from_info(names: Names, info: &FuncInfo) -> Self {
        let mut obj = Self {
            opt_args_ty_name: if info.has_optional_args {
                names.arg_struct_name()
            } else {
                String::new()
            },
            names,
            ..Default::default()
        };
        obj.init(info);
        obj
    }

    fn init(&mut self, info: &FuncInfo) {
        self.do_fn_return(info);

        if matches!(info.func_type, Destructor) {
            self.func_pre_call
                .push(quote! { drop(Box::from_raw(*__objp)); *__objp = ::std::ptr::null_mut(); });
        } else {
            self.func_pre_call
                .push(quote! { let mut __ctx = Ctx::from_ptr(__ctx); });
            self.wrap_fn_arg_decl.push(quote! { __ctx: *mut vrt_ctx });
            self.cproto_fn_arg_decl.push("VRT_CTX".to_string());
        }
        if matches!(info.func_type, Constructor | Destructor) {
            let obj_name = self.names.obj_name().to_ident();
            self.wrap_fn_arg_decl
                .push(quote! { __objp: *mut *mut #obj_name });
            self.cproto_fn_arg_decl
                .push(format!("{} **", self.names.struct_obj_name()));
        }
        if matches!(info.func_type, Constructor) {
            self.wrap_fn_arg_decl
                .push(quote! { __vcl_name: *const c_char });
            self.cproto_fn_arg_decl.push("const char *".to_string());
        }
        if matches!(info.func_type, Method) {
            let obj_name = self.names.obj_access();
            self.wrap_fn_arg_decl
                .push(quote! { __obj: *const #obj_name });
            self.cproto_fn_arg_decl
                .push(format!("{} *", self.names.struct_obj_name()));
        }
        if matches!(info.func_type, Event) {
            self.wrap_fn_arg_decl.push(quote! { __vp: *mut vmod_priv });
            self.wrap_fn_arg_decl.push(quote! { __ev: VclEvent });
        }
        if info.has_optional_args {
            self.func_pre_call
                .push(quote! { let __args = __args.as_ref().unwrap(); });
            let ty = self.opt_args_ty_name.to_ident();
            self.wrap_fn_arg_decl.push(quote! { __args: *const #ty });
        }

        for arg in &info.args {
            self.do_fn_param(info, arg);
        }

        let wrapper_fn_name = self.names.wrapper_fn_name().to_ident();
        let signature = self.get_wrapper_fn_sig(false);

        self.export_decl = quote! { #wrapper_fn_name: Option< #signature > };
        self.export_init = quote! { #wrapper_fn_name: Some(#wrapper_fn_name) };
        self.wrapper_function_body = self.gen_callback_fn(info);
        (self.cproto_typedef_init, self.cproto_typedef_decl) = self.gen_cproto(info);
        self.json = self.json_func(info);
    }

    /// per-function part of $CPROTO - returns typedef init (part of common struct) and declaration code
    fn gen_cproto(&self, info: &FuncInfo) -> (String, String) {
        let (td_name, decl) = match &info.func_type {
            Function | Constructor | Destructor | Method => {
                let mut decl = "\n".to_string();
                if info.has_optional_args {
                    // This corresponds to the Rust declaration created in `gen_callback_fn`
                    // struct arg_vmod_example_captain_obvious {
                    //    char    valid_n;
                    //    VCL_INT n;
                    // };
                    // typedef VCL_STRING td_vmod_example_captain_obvious(VRT_CTX, struct arg_vmod_example_captain_obvious*);
                    let _ = writeln!(decl, "struct {} {{", self.opt_args_ty_name);
                    for arg in &self.cproto_opt_arg_decl {
                        let _ = writeln!(decl, "  {arg};");
                    }
                    let _ = writeln!(decl, "}};\n");
                }

                let _ = write!(
                    decl,
                    "typedef {} {}(",
                    self.output_hdr,
                    self.names.typedef_name()
                );

                for (idx, arg) in self.cproto_fn_arg_decl.iter().enumerate() {
                    if idx != 0 {
                        decl.push(',');
                    }
                    decl.push_str("\n    ");
                    decl.push_str(arg);
                }

                if info.has_optional_args {
                    let _ = write!(decl, ",\n    struct {} *", self.opt_args_ty_name);
                } else {
                    for arg in &self.cproto_wrapper_args {
                        let _ = write!(decl, ",\n    {arg}");
                    }
                }
                let _ = writeln!(decl, "\n);");

                (self.names.typedef_name(), decl)
            }
            Event => ("vmod_event_f".to_string(), String::new()),
        };

        (format!("  {td_name} *{};\n", self.names.f_fn_name()), decl)
    }

    #[allow(clippy::too_many_lines)]
    fn do_fn_param(&mut self, func_info: &FuncInfo, arg_info: &ParamTypeInfo) {
        let arg_name_ident = arg_info.ident.to_ident();
        let temp_var = format_ident!("__var{}", arg_info.idx);

        // Access to the input value, either from the args struct or directly
        let input_val = if func_info.has_optional_args {
            quote! { __args.#arg_name_ident }
        } else {
            quote! { #arg_name_ident }
        };

        match &arg_info.ty {
            ParamType::Context { is_mut } => {
                let ident = "__ctx".to_ident();
                self.func_call_vars.push(if *is_mut {
                    quote! { &mut #ident }
                } else {
                    quote! { &#ident }
                });
            }
            ParamType::SelfType => {
                self.func_pre_call
                    .push(quote! { let __obj = __obj.as_ref().unwrap(); });
            }
            ParamType::Event => {
                self.func_call_vars.push(quote! { __ev });
                let json = Self::arg_to_json(arg_info.ident.clone(), false, "EVENT", Value::Null);
                self.args_json.push(json.into());
            }
            ParamType::VclName => {
                self.func_pre_call
                    .push(quote! { let #temp_var: Cow<'_, str> = VCL_STRING(__vcl_name).into(); });
                self.func_call_vars.push(quote! { &#temp_var });
            }
            ParamType::SharedPerTask => {
                self.add_wrapper_arg(func_info, quote! { #arg_name_ident: *mut vmod_priv });
                self.func_pre_call
                    .push(quote! { let mut #temp_var = (* #input_val).take(); });
                self.func_call_vars.push(quote! { &mut #temp_var });
                self.func_post_call.push(quote! {
                    // Release ownership back to Varnish
                    if let Some(obj) = #temp_var {
                        (* #input_val).put(obj, &PRIV_TASK_METHODS);
                    }
                });

                let json =
                    Self::arg_to_json(arg_info.ident.clone(), false, "PRIV_TASK", Value::Null);
                self.args_json.push(json.into());
                self.add_cproto_arg(func_info, "struct vmod_priv *", &arg_info.ident);
            }
            ParamType::SharedPerVclRef => {
                self.add_wrapper_arg(func_info, quote! { #arg_name_ident: *const vmod_priv });
                self.func_pre_call.push(quote! {
                    // defensive programming: *vmod_priv should never be NULL,
                    // but might as well just treat it as None rather than crashing - its readonly anyway
                    let #temp_var = #input_val.as_ref().and_then(|v| v.get_ref());
                });
                self.func_call_vars.push(quote! { #temp_var });
                let json =
                    Self::arg_to_json(arg_info.ident.clone(), false, "PRIV_VCL", Value::Null);
                self.args_json.push(json.into());
                self.add_cproto_arg(func_info, "struct vmod_priv *", &arg_info.ident);
            }
            ParamType::SharedPerVclMut => {
                self.add_wrapper_arg(func_info, quote! { #arg_name_ident: *mut vmod_priv });
                let input_val = if matches!(func_info.func_type, Event) {
                    quote! { __vp } // Event input vars are hardcoded (for now), use `__vp`
                } else {
                    input_val
                };
                self.func_pre_call
                    .push(quote! { let mut #temp_var = (* #input_val).take(); });
                self.func_call_vars.push(quote! { &mut #temp_var });
                self.func_post_call.push(quote! {
                    // Release ownership back to Varnish
                    if let Some(obj) = #temp_var {
                        (* #input_val).put(obj, &PRIV_VCL_METHODS);
                    }
                });
                let json =
                    Self::arg_to_json(arg_info.ident.clone(), false, "PRIV_VCL", Value::Null);
                self.args_json.push(json.into());
                self.add_cproto_arg(func_info, "struct vmod_priv *", &arg_info.ident);
            }
            ParamType::Value(pi) => {
                // Convert C arg into Rust arg and pass it to the user's function
                let mut input_expr = quote! { #input_val.into() };
                let mut temp_var_ty = pi.ty_info.to_rust_type();
                if pi.is_optional {
                    let input_valid = format_ident!("valid_{}", arg_info.ident);
                    if !pi.ty_info.must_be_optional() {
                        input_expr = quote! { Some(#input_expr) };
                        // else input_expr will be converted to option as is
                    }
                    input_expr =
                        quote! { if __args.#input_valid != 0 { #input_expr } else { None } };
                    self.add_wrapper_arg(func_info, quote! { #input_valid: c_char });
                    self.cproto_opt_arg_decl.push(format!("char {input_valid}"));
                }
                if pi.is_optional || pi.ty_info.must_be_optional() {
                    temp_var_ty = quote! { Option<#temp_var_ty> };
                }

                let mut init_var = quote! { let #temp_var: #temp_var_ty = #input_expr; };

                // For `str`, we now have a Cow or an Option<Cow> which need additional parsing.
                // We cannot do this on the same statement because we need a ref to it without dropping the temp var.
                if matches!(pi.ty_info, ParamTy::Str) {
                    if pi.is_optional {
                        init_var = quote! { #init_var let #temp_var = #temp_var.as_deref(); };
                    } else {
                        init_var = quote! { #init_var let #temp_var = #temp_var.as_ref(); };
                    }
                }
                self.func_pre_call.push(init_var);

                let c_type = pi.ty_info.to_c_type().to_ident();
                self.add_wrapper_arg(func_info, quote! { #arg_name_ident: #c_type });
                self.func_call_vars.push(quote! { #temp_var });

                let json = Self::arg_to_json(
                    arg_info.ident.clone(),
                    pi.is_optional,
                    pi.ty_info.to_vcc_type(),
                    pi.default.clone(),
                );
                self.args_json.push(json.into());
                self.add_cproto_arg(func_info, pi.ty_info.to_c_type(), &arg_info.ident);
            }
        };
    }

    fn add_wrapper_arg(&mut self, func_info: &FuncInfo, code: TokenStream) {
        // Events do not modify their signature
        if matches!(func_info.func_type, Function | Constructor | Method) {
            if func_info.has_optional_args {
                self.opt_args_arg_decl.push(code);
            } else {
                self.wrap_fn_arg_decl.push(code);
            }
        }
    }

    fn add_cproto_arg(&mut self, func_info: &FuncInfo, ctype: &'static str, arg_name: &str) {
        if func_info.has_optional_args {
            self.cproto_opt_arg_decl.push(format!("{ctype} {arg_name}"));
        } else {
            self.cproto_wrapper_args.push(ctype);
        }
    }

    fn arg_to_json(
        arg_name: String,
        is_optional_arg: bool,
        vcc_type: &str,
        default: Value,
    ) -> Vec<Value> {
        // JSON data for each argument:
        //   [VCC_type, arg_name, default_value, spec(?), is_optional]
        let default = if default == Value::Null {
            Value::Null
        } else {
            // This ensures the string is properly escaped and surrounded by quotes
            default.to_string().into()
        };
        let mut json_arg: Vec<Value> = vec![
            vcc_type.into(),
            arg_name.into(),
            default,
            Value::Null, // spec param is not used at this point
        ];

        if is_optional_arg {
            json_arg.push(true.into());
        } else {
            // trim all NULLs from the end of json_arg list
            while let Some(Value::Null) = json_arg.last() {
                json_arg.pop();
            }
        }

        json_arg
    }

    fn do_fn_return(&mut self, info: &FuncInfo) {
        let ty = if matches!(info.func_type, Event) {
            // Rust event functions do not return value, but their C wrappers must return an int
            &ReturnTy::ParamType(ParamTy::I64)
        } else {
            info.returns.value_type()
        };
        self.output_hdr = ty.to_c_type();
        self.wrap_fn_output = if self.output_hdr == "VCL_VOID" {
            quote! {}
        } else {
            let ident = self.output_hdr.to_ident();
            quote! { -> #ident }
        };
        self.output_vcc = ty.to_vcc_type();
    }

    fn json_func(&self, info: &FuncInfo) -> Value {
        let callback_fn = format!(
            "{}.{}",
            self.names.func_struct_name(),
            self.names.f_fn_name()
        );
        let args_struct_cproto = if info.has_optional_args {
            format!("struct {}", self.opt_args_ty_name)
        } else {
            String::new()
        };
        let mut decl: Vec<Value> = vec![
            vec![self.output_vcc.clone()].into(),
            callback_fn.clone().into(),
            args_struct_cproto.into(),
        ];
        decl.extend(self.args_json.iter().cloned());

        match info.func_type {
            Function | Method => {
                json! { [ info.func_type.to_vcc_type(), self.names.fn_name().to_string(), decl ] }
            }
            Constructor | Destructor => {
                json! { [ info.func_type.to_vcc_type(), decl ] }
            }
            Event => {
                json! { [ info.func_type.to_vcc_type(), callback_fn ] }
            }
        }
    }

    /// Generate an extern "C" wrapper function that calls user's Rust function
    fn gen_callback_fn(&self, info: &FuncInfo) -> TokenStream {
        let opt_param_struct = self.gen_opt_param_struct(info);
        let signature = self.get_wrapper_fn_sig(true);
        let func_pre_call = &self.func_pre_call;
        let call_user_fn = self.gen_user_fn_call(info);
        let func_post_call = &self.func_post_call;
        let unwrap_result = self.gen_result_handler_code(info);

        quote! {
            #opt_param_struct
            #signature {
                #(#func_pre_call)*
                #call_user_fn
                #(#func_post_call)*
                #unwrap_result
            }
        }
    }

    /// Get the signature of the wrapper function:  `unsafe extern "C" fn name(ctx: *mut vrt_ctx, ...) -> VCL_STRING`
    /// If `with_fn_name` is true, the function name is included in the signature
    fn get_wrapper_fn_sig(&self, with_fn_name: bool) -> TokenStream {
        let fn_name = with_fn_name.then(|| self.names.wrapper_fn_name().to_ident());
        let fn_args = &self.wrap_fn_arg_decl;
        let fn_output = &self.wrap_fn_output;
        quote! { unsafe extern "C" fn #fn_name(#(#fn_args),*) #fn_output }
    }

    /// Get the Rust args struct in case optional arguments are being used
    fn gen_opt_param_struct(&self, info: &FuncInfo) -> TokenStream {
        if info.has_optional_args {
            let opt_args_ty_name = self.opt_args_ty_name.to_ident();
            let opt_args_arg_decl = &self.opt_args_arg_decl;
            quote! {
                #[repr(C)]
                struct #opt_args_ty_name {
                    #(#opt_args_arg_decl,)*
                }
            }
        } else {
            quote! {}
        }
    }

    fn gen_user_fn_call(&self, info: &FuncInfo) -> TokenStream {
        if let Destructor = info.func_type {
            return quote! {};
        }

        let user_fn_name = self.names.fn_callable_name(info.func_type);
        let var_args = &self.func_call_vars;
        let call_user_fn = quote! {
            let __result = #user_fn_name(#(#var_args),*);
        };
        call_user_fn
    }

    fn gen_result_handler_code(&self, info: &FuncInfo) -> TokenStream {
        let is_result;
        let on_error = match &info.returns {
            ReturnType::Result(_, err) => {
                is_result = true;
                let err = if matches!(err, ReturnTy::BoxDynError) {
                    quote! { &err.to_string() }
                } else {
                    quote! { err }
                };
                quote! { __ctx.fail(#err); }
            }
            ReturnType::Value(_) => {
                is_result = false;
                quote! {}
            }
        };
        let is_vcl_type = matches!(info.returns.value_type(), ReturnTy::VclType(_));

        // Events require special handling - convert errors into 1, otherwise 0
        if matches!(info.func_type, Event) {
            return if is_result {
                quote! {
                    match __result {
                        Ok(_) => VCL_INT(0),
                        Err(err) => { #on_error; VCL_INT(1) },
                    }
                }
            } else {
                quote! { VCL_INT(0) }
            };
        }

        let default_expr = if self.output_hdr == "VCL_VOID" {
            quote! {}
        } else {
            quote! { Default::default() }
        };

        let on_success = if matches!(info.func_type, Constructor) {
            quote! {
                let __result = Box::new(__result);
                *__objp = Box::into_raw(__result);
            }
        } else if self.output_hdr == "VCL_VOID" {
            quote! {}
        } else if is_vcl_type {
            quote! { __result }
        } else {
            quote! {
                match __result.into_vcl(&mut __ctx.ws) {
                    Ok(v) => v,
                    Err(err) => {
                        __ctx.fail(err);
                        #default_expr
                    }
                }
            }
        };

        if is_result {
            quote! {
                match __result {
                    Ok(__result) => {
                        #on_success
                    },
                    Err(err) => {
                        #on_error;
                        #default_expr
                    }
                }
            }
        } else {
            on_success
        }
    }
}
