use core::fmt;

#[derive(Debug, Clone)]
pub struct ManifestError(pub String);

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "manifest error: {}", self.0)
    }
}