use std::collections::HashMap;
use std::str::FromStr;
use crate::error::{ManifestError};
use crate::typed_value::Type;

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i32,
    pub data_type: i32,
    pub string: String,
}

#[derive(Clone)]
pub struct Manifest {
    entries: Vec<Entry>,
    entries_by_path: HashMap<String, Entry>,
}

impl Manifest {
    pub(crate) fn from_str(manifest_str: &str) -> Self {
        let mut entries = Vec::<Entry>::new();
        for line in manifest_str.split('\n') {
            if line.trim().is_empty() { continue }
            let mut line_split = line.split(',');

            let id_str = line_split.next().unwrap();
            let type_str = line_split.next().unwrap();
            let string = line_split.next().unwrap();

            entries.push(Entry {
                id: i32::from_str(id_str).unwrap(),
                data_type: i32::from_str(type_str).unwrap(),
                string: string.to_string()
            });
        }

        Self {
            entries: entries.clone(),
            entries_by_path: Self::construct_entries_by_path(&entries),
        }
    }

    pub fn construct_entries_by_path(entries: &Vec<Entry>) -> HashMap<String, Entry> {
        let mut result = HashMap::new();
        for entry in entries {
            result.insert(entry.string.clone(), entry.clone());
        }

        result
    }

    pub(crate) fn get_entry_by_id(&self, id: &i32) -> Result<&Entry, ManifestError> {
        for entry in &self.entries {
            if &entry.id == id {
                return Ok(entry)
            }
        }

        Err(ManifestError::NoSuchEntryId(*id))
    }

    pub(crate) fn get_entry_by_path(&self, path: &str) -> Result<&Entry, ManifestError> {
        let entry = self.entries_by_path.get(path);

        if entry.is_some() {
            Ok(entry.unwrap())
        } else {
            Err(ManifestError::NoSuchEntryPath(path.to_string()))
        }
    }

    pub fn get_entries_with_prefix(&self, prefix: &str) -> Vec<&Entry> {
        let mut result = Vec::new();
        for entry in &self.entries_by_path {
            if entry.0.starts_with(prefix) {
                result.push(entry.1);
            }
        }

        result
    }

    pub(crate) fn get_data_type_for_id(&self, id: &i32) -> Result<Type, ManifestError> {
        let entry = self.get_entry_by_id(id);
        if entry.is_err() {
            return Err(ManifestError::NoSuchEntryId(*id))
        }

        let act_entry = entry.unwrap();
        match act_entry.data_type {
            0 => Ok(Type::Boolean),
            1 => Ok(Type::Integer32),
            2 => Ok(Type::Float),
            3 => Ok(Type::Double),
            4 => Ok(Type::String),
            5 => Ok(Type::Long),
            _ => Err(ManifestError::WrongDataType(act_entry.data_type))
        }
    }

    pub fn entries_num(&self) -> usize {
        self.entries.len()
    }
}