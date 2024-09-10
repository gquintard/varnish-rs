use crate::ffi::{VCL_BOOL, VCL_INT, VCL_REAL};

macro_rules! impl_type_cast_from_typ {
    ($ident:ident, $typ:ty) => {
        impl From<$typ> for $ident {
            fn from(b: $typ) -> Self {
                Self(b.into())
            }
        }
    };
}

macro_rules! impl_type_cast_from_vcl {
    ($ident:ident, $typ:ty) => {
        impl From<$ident> for $typ {
            fn from(b: $ident) -> Self {
                <Self>::from(b.0)
            }
        }
    };
}

impl_type_cast_from_vcl!(VCL_REAL, f64);
impl_type_cast_from_typ!(VCL_REAL, f64);

impl_type_cast_from_typ!(VCL_INT, i64);
impl_type_cast_from_vcl!(VCL_INT, i64);

impl_type_cast_from_typ!(VCL_BOOL, bool);
impl From<VCL_BOOL> for bool {
    fn from(b: VCL_BOOL) -> Self {
        b.0 != 0
    }
}
