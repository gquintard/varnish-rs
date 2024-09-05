/// custom vcl `Error` type
///
/// The C errors aren't typed and are just C strings, so we just wrap them into a proper rust
/// `Error`
pub struct Error {
    s: String,
}

impl Error {
    /// Create a new `Error` from a string
    pub(crate) fn new(s: String) -> Self {
        Error { s }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.s, f)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.s, f)
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error { s }
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error { s: s.into() }
    }
}

/// Shorthand to [`std::result::Result<T, Error>`]
pub type Result<T> = std::result::Result<T, Error>;
