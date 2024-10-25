//! The data model is the validated result of parsing user code.
//! Once fully parsed and vetted, the data model is used to generate the Varnish VMOD code.

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;

/// Represents the entire VMOD. A single instance of this struct is parsed for each VMOD.
#[derive(Debug, Default)]
pub struct VmodInfo {
    pub params: VmodParams,
    pub ident: String,
    pub docs: String,
    pub funcs: Vec<FuncInfo>,
    pub objects: Vec<ObjInfo>,
    pub shared_types: SharedTypes,
}

/// Represents the shared types used by multiple functions. Each of these types is unique per VMOD.
#[derive(Debug, Default)]
pub struct SharedTypes {
    pub shared_per_task_ty: Option<String>,
    pub shared_per_vcl_ty: Option<String>,
}

/// Represents the parameters inside the `#[vmod(....)]` attribute itself.
#[derive(Default, Debug, FromMeta)]
#[darling(default)]
pub struct VmodParams {
    pub docs: Option<String>,
}

/// Represents the object information parsed from an `impl` block.
#[derive(Debug)]
pub struct ObjInfo {
    pub ident: String,
    pub docs: String,
    pub constructor: FuncInfo,
    pub destructor: FuncInfo,
    pub funcs: Vec<FuncInfo>,
}

/// Represents the function information parsed from a function or method.
#[derive(Debug)]
pub struct FuncInfo {
    pub func_type: FuncType,
    pub ident: String,
    pub docs: String,
    pub has_optional_args: bool,
    pub args: Vec<ParamTypeInfo>,
    pub returns: ReturnType,
}

/// What kind of function is this?
#[derive(Debug, Clone, Copy, Default)]
pub enum FuncType {
    #[default]
    Function,
    Constructor,
    Destructor,
    Method,
    Event,
}

impl FuncType {
    pub fn to_vcc_type(self) -> &'static str {
        match self {
            Self::Function => "$FUNC",
            Self::Constructor => "$INIT",
            Self::Destructor => "$FINI",
            Self::Method => "$METHOD",
            Self::Event => "$EVENT",
        }
    }
}

/// Represents the return type of function - either the value itself, or a `Result` with an error.
#[derive(Debug, Clone)]
pub enum ReturnType {
    Value(ReturnTy),
    Result(ReturnTy, ReturnTy),
}

impl ReturnType {
    pub fn value_type(&self) -> ReturnTy {
        match self {
            Self::Value(ty) | Self::Result(ty, _) => *ty,
        }
    }
}

/// Represents the information about a single function argument.
#[derive(Debug)]
pub struct ParamTypeInfo {
    pub ident: String,
    pub docs: String,
    pub idx: usize,
    pub ty: ParamType,
}

/// Represents the type of the function argument.
#[derive(Debug, Clone)]
pub enum ParamType {
    /// An argument representing Varnish context (VRT_CTX) wrapper
    Context { is_mut: bool },
    /// For object methods, the first argument is always a reference to the object
    SelfType,
    /// An argument is an event type
    Event,
    /// A `&str` argument automatically passed for object creation representing a VCL name
    VclName,
    /// An argument `&mut Option<Box<T>>` representing any Rust name and type shared across tasks (i.e. `PRIV_TASK`)
    SharedPerTask,
    /// A readonly argument `Option<&T>` representing any Rust name and type shared across VCL load (i.e. `PRIV_VCL`)
    SharedPerVclRef,
    /// A mutable argument `&mut Option<Box<T>>` representing any Rust name and type shared across VCL load (i.e. `PRIV_VCL`)
    SharedPerVclMut,
    /// An argument representing a basic VCL type
    Value(ParamInfo),
}

/// Represents the information about the common function argument types
#[derive(Debug, Clone)]
pub struct ParamInfo {
    pub is_optional: bool,
    pub default: serde_json::Value,
    pub ty_info: ParamTy,
}

/// Represents the common function argument types. These could also be returned.
#[derive(Debug, Clone, Copy)]
pub enum ParamTy {
    Bool,
    Duration,
    F64,
    I64,
    Probe, // FIXME: can probes be returned?
    ProbeCow,
    SocketAddr,
    Str,
}

impl ParamTy {
    pub fn to_rust_type(self) -> TokenStream {
        match self {
            Self::Bool => quote! { bool },
            Self::Duration => quote! { Duration },
            Self::F64 => quote! { f64 },
            Self::I64 => quote! { i64 },
            Self::Probe => quote! { Probe },
            Self::ProbeCow => quote! { COWProbe },
            Self::SocketAddr => quote! { SocketAddr },
            Self::Str => quote! { Cow<'_, str> },
        }
    }
}

impl ParamTy {
    pub fn to_vcc_type(self) -> &'static str {
        match self {
            Self::Bool => "BOOL",
            Self::Duration => "DURATION",
            Self::F64 => "REAL",
            Self::I64 => "INT",
            Self::Probe | Self::ProbeCow => "PROBE",
            Self::SocketAddr => "IP",
            Self::Str => "STRING",
        }
    }

    pub fn to_c_type(self) -> &'static str {
        // ATTENTION: Each VCL_* type here must also be listed in the `use varnish::...`
        //            statement in the `varnish-macros/src/generator.rs` file.
        match self {
            Self::Bool => "VCL_BOOL",
            Self::Duration => "VCL_DURATION",
            Self::F64 => "VCL_REAL",
            Self::I64 => "VCL_INT",
            Self::Probe | Self::ProbeCow => "VCL_PROBE",
            Self::SocketAddr => "VCL_IP",
            Self::Str => "VCL_STRING",
        }
    }

    pub fn must_be_optional(self) -> bool {
        match self {
            // &str is a special case(?), it can be non-optional
            Self::Bool | Self::Duration | Self::F64 | Self::I64 | Self::Str => false,
            Self::Probe | Self::ProbeCow | Self::SocketAddr => true,
        }
    }
}

/// Represents all return types of functions.
#[derive(Debug, Clone, Copy)]
pub enum ReturnTy {
    Default, // Nothing is returned
    SelfType,
    ParamType(ParamTy),
    String,
    Backend,
    Bytes,
    VclString,   // hopefully some day we won't expose this type to the user
    BoxDynError, // Error type only
    VclError,    // Error type only
}

impl ReturnTy {
    pub fn to_vcc_type(self) -> &'static str {
        match self {
            // Self is returned by obj constructors which are void in VCC
            Self::Default | Self::SelfType => "VOID",
            Self::ParamType(ty) => ty.to_vcc_type(),
            Self::Bytes | Self::VclString | Self::String => "STRING",
            Self::Backend => "BACKEND",
            Self::BoxDynError | Self::VclError => "VCC-SomeError", // internal to the generator
        }
    }

    pub fn to_c_type(self) -> &'static str {
        // ATTENTION: Each VCL_* type here must also be listed in the `use varnish::...`
        //            statement in the `varnish-macros/src/generator.rs` file.
        match self {
            Self::ParamType(ty) => ty.to_c_type(),
            Self::Bytes | Self::VclString | Self::String => "VCL_STRING",
            Self::SelfType | Self::Default => "VCL_VOID",
            Self::Backend => "VCL_BACKEND",
            Self::BoxDynError | Self::VclError => "C-BoxDynError", // internal to the generator
        }
    }
}
