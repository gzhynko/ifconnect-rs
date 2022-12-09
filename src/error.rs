use core::fmt;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum ManifestError{
    NoSuchEntryId(i32),
    NoSuchEntryPath(String),
    WrongDataType(i32),
    NoManifest(),
}

impl Error for ManifestError {}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ManifestError::NoSuchEntryId(id) => write!(f, "Manifest error: no entry with id: {}", id),
            ManifestError::NoSuchEntryPath(path) => write!(f, "Manifest error: no entry with path: {}", path),
            ManifestError::WrongDataType(data_type) => write!(f, "Manifest error: entry has unknown data type: {}", data_type),
            ManifestError::NoManifest() => write!(f, "Manifest error: manifest has not been retrieved yet"),
        }
    }
}
