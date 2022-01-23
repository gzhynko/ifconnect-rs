use core::fmt;

#[derive(Debug, Clone)]
pub enum ManifestError{
    NoSuchEntryId(i32),
    NoSuchEntryPath(String),
    WrongDataType(i32),
    NoManifest(),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ManifestError::NoSuchEntryId(id) => write!(f, "manifest error: no entry with id: {}", id),
            ManifestError::NoSuchEntryPath(path) => write!(f, "manifest error: no entry with path: {}", path),
            ManifestError::WrongDataType(data_type) => write!(f, "manifest error: entry has unknown data type: {}", data_type),
            ManifestError::NoManifest() => write!(f, "manifest error: manifest has not been retrieved yet"),
        }
    }
}