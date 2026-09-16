use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Data {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Data>),
    Object(Vec<(String, Data)>),
    Hash(HashMap<String, Data>),
}

impl From<String> for Data {
    fn from(value: String) -> Self {
        Data::String(value)
    }
}

impl From<&str> for Data {
    fn from(value: &str) -> Self {
        Data::String(value.to_string())
    }
}

impl From<bool> for Data {
    fn from(value: bool) -> Self {
        Data::Bool(value)
    }
}

impl From<i64> for Data {
    fn from(value: i64) -> Self {
        Data::Int(value)
    }
}

impl From<i32> for Data {
    fn from(value: i32) -> Self {
        Data::Int(value as i64)
    }
}

impl From<f64> for Data {
    fn from(value: f64) -> Self {
        Data::Float(value)
    }
}

impl From<f32> for Data {
    fn from(value: f32) -> Self {
        Data::Float(value as f64)
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Data::Bytes(value)
    }
}
impl From<HashMap<String, Data>> for Data {
    fn from(value: HashMap<String, Data>) -> Self {
        Data::Hash(value)
    }
}

impl<T: Into<Data>> From<Vec<T>> for Data {
    fn from(value: Vec<T>) -> Self {
        Data::Array(value.into_iter().map(Into::into).collect())
    }
}