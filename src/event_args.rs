use crate::manifest::Manifest;
use crate::typed_value::TypedValue;

pub struct ReceivedDataArgs {
    pub command_id: i32,
    pub data: TypedValue,
}

impl ReceivedDataArgs {
    pub fn new(command_id: i32, data: TypedValue) -> Self {
        Self {
            command_id,
            data,
        }
    }
}

pub struct ReceivedManifestArgs {
    pub manifest: Manifest,
}

impl ReceivedManifestArgs {
    pub fn new(manifest: Manifest) -> Self {
        Self {
            manifest,
        }
    }
}
