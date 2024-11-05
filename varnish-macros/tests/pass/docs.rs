#![allow(unused_variables)]

use varnish::vmod;

fn main() {}

/// doctest for `DocStruct`.
/// This comment is ignored because we do not handle structs, only impl blocks.
pub struct DocStruct;

/// main docs
/// # Big header
/// ## sub header
/// foo bar
#[vmod]
mod types {
    /*! Multiline
     * comment per https://github.com/rust-lang/rust/issues/32088
     *
     * The end
     */

    use super::DocStruct;

    /// doctest on a function
    /// with multiple lines
    /// # Big header
    /// ## sub header
    /// * foo
    /// * bar
    pub fn with_docs() {}

    pub fn no_docs() {}

    /// doctest on a function
    pub fn doctest(
        // param without docs
        _no_docs: i64,
        /// doc comment on *function arguments* are invalid in Rust,
        /// but they are parsed by macros.
        // This comment is not parsed by `#[doc]` attribute,
        /// we can generate documentation for param `_v` here.
        ///
        /// ## Example
        /// This comment is multi-lined to ensure multiple `#[doc]` are parsed correctly.
        _v: i64,
    ) {
    }

    pub fn arg_only(
        /// doc comment for `arg_only`
        _v: i64,
    ) {
    }

    /// doctest for `DocStruct` implementation
    impl DocStruct {
        /// doctest for `new`
        pub fn new(
            /// doc comment for `cap`
            cap: Option<i64>,
        ) -> Self {
            DocStruct
        }
        /// doctest for the object function
        #[rustfmt::skip]
        pub fn function(
            /// self docs - note that `rustfmt` will put `&self` right after this comment
            /// on the same line, effectively removing the code!
            /// We cannot really document it anyway, so we parse it but skip it later.
            &self,
            /// param docs
            key: &str,
        ) {
        }
    }
}
