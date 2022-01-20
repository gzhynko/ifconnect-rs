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
        let mut vec_arr = Vec::<u8>::new();
        match self {
            Self::Boolean(val) => vec_arr = Vec::from((*val as i32).to_be_bytes()),
            Self::Integer32(val) => vec_arr = Vec::from(val.to_be_bytes()),
            Self::Float(val) => vec_arr = Vec::from(val.to_be_bytes()),
            Self::Double(val) => vec_arr = Vec::from(val.to_be_bytes()),
            Self::String(val) => vec_arr = val.to_owned().into_bytes(),
            Self::Long(val) => vec_arr = Vec::from(val.to_be_bytes()),
            _ => {}
        }

        vec_arr
    }
}
