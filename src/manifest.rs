use std::str::FromStr;
use crate::error::{ManifestError};
use crate::typed_value::Type;

#[derive(Debug, Clone)]
pub struct Entry {
    id: i32,
    data_type: i32,
    string: String,
}

pub struct Manifest {
    entries: Vec<Entry>
}

impl Manifest {
    pub(crate) fn from_str(manifest_str: &str) -> Self {
        let mut manifest = Vec::<Entry>::new();
        for line in manifest_str.split('\n') {
            let mut line_split = line.split(',');

            let id_str = line_split.next().unwrap();
            let type_str = line_split.next().unwrap();
            let string = line_split.next().unwrap();

            manifest.push(Entry {
                id: i32::from_str(&id_str).unwrap(),
                data_type: i32::from_str(&type_str).unwrap(),
                string: string.to_string()
            });
        }

        Self {
            entries: manifest
        }
    }

    pub(crate) fn get_entry_by_id(&self, id: &i32) -> Result<Entry, ManifestError> {
        for entry in &self.entries {
            if &entry.id == id {
                return Ok(entry.clone())
            }
        }

        return Err(ManifestError(String::from(format!("no entry with the following id found: {}", &id))))
    }

    pub(crate) fn get_data_type(&self, id: &i32) -> Result<Type, ManifestError> {
        let entry = self.get_entry_by_id(&id);
        if entry.is_err() {
            return Err(ManifestError(String::from(format!("no entry with the following id found: {}", &id))))
        }

        let act_entry = entry.unwrap();
        match act_entry.data_type {
            0 => Ok(Type::Boolean),
            1 => Ok(Type::Integer32),
            2 => Ok(Type::Float),
            3 => Ok(Type::Double),
            4 => Ok(Type::String),
            5 => Ok(Type::Long),
            _ => Err(ManifestError(String::from(format!("entry has wrong data type: {}", act_entry.data_type))))
        }
    }
}