use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub enum TypedValue {
    Boolean(bool),
    Integer32(i32),
    Float(f32),
    Double(f64),
    String(String),
    Long(i64)
}

#[derive(PartialEq)]
pub enum Type {
    Boolean,
    Integer32,
    Float,
    Double,
    String,
    Long
}

impl TypedValue {
    pub fn to_bytes_vec(&self) -> Vec<u8> {
        let vec_arr;
        match self {
            Self::Boolean(val) => vec_arr = Vec::from((*val as i32).to_le_bytes()),
            Self::Integer32(val) => vec_arr = Vec::from(val.to_le_bytes()),
            Self::Float(val) => vec_arr = Vec::from(val.to_le_bytes()),
            Self::Double(val) => vec_arr = Vec::from(val.to_le_bytes()),
            Self::String(val) => vec_arr = val.to_owned().into_bytes(),
            Self::Long(val) => vec_arr = Vec::from(val.to_le_bytes()),
        }

        vec_arr
    }
}

impl Display for TypedValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Self::Boolean(val) => val.to_string(),
            Self::Integer32(val) => val.to_string(),
            Self::Float(val) => val.to_string(),
            Self::Double(val) => val.to_string(),
            Self::String(val) => val.to_string(),
            Self::Long(val) => val.to_string(),
        })
    }
}
