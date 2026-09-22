use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use serde::Serialize;
use crate::models::data::Data;
pub fn write<T: Into<Data>>(folder: PathBuf, file_name: String, data: T) -> Result<(), String> {
    let mut path = folder.join(&file_name);
    if path.extension().is_none() {
        path.set_extension("glh");
    }
    let encoded_data = encode(data.into());
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    file.write_all(&encoded_data).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn write_typed<T: Serialize + ?Sized>(folder: PathBuf, file_name: String, data: &T) -> Result<(), String> {
    let json_value = serde_json::to_value(data).map_err(|e| e.to_string())?;
    let converted = json_to_data(json_value);
    let mut path = folder.join(&file_name);
    if path.extension().is_none() {
        path.set_extension("glh");
    }
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    file.write_all(&encode(converted)).map_err(|e| e.to_string())?;
    Ok(())
}
fn json_to_data(value: serde_json::Value) -> Data {
    match value {
        serde_json::Value::Null => Data::Null,
        serde_json::Value::Bool(b) => Data::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Data::Int(i)
            } else {
                Data::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => Data::String(s),
        serde_json::Value::Array(arr) => Data::Array(arr.into_iter().map(json_to_data).collect()),
        serde_json::Value::Object(map) => Data::Object(
            map.into_iter().map(|(k, v)| (k, json_to_data(v))).collect()
        ),
    }
}
pub fn serialize_to_bytes<T: Into<Data>>(data: T) -> Vec<u8> {
    encode(data.into())
}
pub fn encode(data: Data) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GLH");
    bytes.push(1);
    bytes.extend_from_slice(&[0; 4]);
    encode_value(&data, &mut bytes);
    let length = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&length.to_le_bytes());
    bytes
}
fn encode_value(data: &Data, bytes: &mut Vec<u8>) {
    match data {
        Data::Null => bytes.push(0),

        Data::Int(value) => {
            bytes.push(1);
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        Data::Bool(value) => {
            bytes.push(2);
            bytes.push(if *value { 1 } else { 0 });
        }

        Data::Float(value) => {
            bytes.push(3);
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        Data::String(value) => {
            bytes.push(4);
            let length = value.len() as u32;
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(value.as_bytes());
        }

        Data::Array(value) => {
            bytes.push(5);
            let length = value.len() as u32;
            bytes.extend_from_slice(&length.to_le_bytes());
            for item in value {
                encode_value(item, bytes);
            }
        }

        Data::Bytes(value) => {
            bytes.push(6);
            let length = value.len() as u32;
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(value);
        }

        Data::Object(value) => {
            bytes.push(7);
            let length = value.len() as u32;
            bytes.extend_from_slice(&length.to_le_bytes());
            for (key, val) in value {
                let key_length = key.len() as u32;
                bytes.extend_from_slice(&key_length.to_le_bytes());
                bytes.extend_from_slice(key.as_bytes());
                encode_value(val, bytes);
            }
        }
        Data::Hash(value) => {
            bytes.push(8);
            let length = value.len() as u32;
            bytes.extend_from_slice(&length.to_le_bytes());
            for (key, val) in value {
                let key_length = key.len() as u32;
                bytes.extend_from_slice(&key_length.to_le_bytes());
                bytes.extend_from_slice(key.as_bytes());
                encode_value(val, bytes);
            }
        }
    }
}