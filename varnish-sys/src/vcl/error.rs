/// custom vcl `Error` type
///
/// The C errors aren't typed and are just C strings, so we just wrap them into a proper rust
/// `Error`
pub struct VclError {
    s: String,
}

impl VclError {
    /// Create a new `Error` from a string
    pub fn new(s: String) -> Self {
        VclError { s }
    }
}

impl std::fmt::Debug for VclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.s, f)
    }
}

impl std::fmt::Display for VclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.s, f)
    }
}

impl std::error::Error for VclError {}

impl From<String> for VclError {
    fn from(s: String) -> Self {
        VclError { s }
    }
}

impl From<&str> for VclError {
    fn from(s: &str) -> Self {
        VclError { s: s.into() }
    }
}

/// Shorthand to [`Result<T, VclError>`]
pub type VclResult<T> = Result<T, VclError>;
