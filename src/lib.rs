use serde::{Deserialize, Serialize}; // трейты serde для JSON-функций ниже
use std::fs::File;                   // работа с файлами на диске
use std::io::{Read as IoRead, Write}; // Read переименован, чтобы не путать с твоими будущими типами
use std::path::{Path, PathBuf};      // Path — заимствованный путь, PathBuf — владеющий (для folder.join)


#[derive(Debug, PartialEq)]
pub enum Data {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Data>),
    Object(Vec<(String, Data)>),
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

impl<T: Into<Data>> From<Vec<T>> for Data {
    fn from(value: Vec<T>) -> Self {
        Data::Array(value.into_iter().map(Into::into).collect())
    }
}

pub fn serialize_to_json<T: Serialize>(data: &T) -> Result<String, String> {
    serde_json::to_string(data).map_err(|e| e.to_string())
}

pub fn deserialize_from_json<'a, T: Deserialize<'a>>(data: &'a str) -> Result<T, String> {
    serde_json::from_str(data).map_err(|e| e.to_string())
}

// Сериализация в твой бинарный формат GLH.
// T: Into<Data> — значит сюда можно передать String, &str, i32, bool, Vec<i32> и т.д. напрямую,
// не оборачивая руками в Data:: — конверсия произойдёт сама через data.into().
// Если нужен Data::Object — его всё равно придётся собирать руками (см. пример ниже кода).
pub fn serialize_to_bytes<T: Into<Data>>(data: T) -> Vec<u8> {
    encode(data.into())
}

// Обратная операция — байты GLH обратно в Data.
pub fn deserialize_from_bytes(data: &[u8]) -> Result<Data, String> {
    decode(data)
}
pub fn write<T: Into<Data>>(folder: PathBuf, file_name: String, data: T) -> Result<(), String> {
    let path = folder.join(file_name);
    let encoded_data = encode(data.into());
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    file.write_all(&encoded_data).map_err(|e| e.to_string())?;
    Ok(())
}


pub fn read(path: &Path) -> Result<Data, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    decode(&buffer)
}


pub fn encode(data: Data) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"GLH");
    bytes.push(1);
    encode_value(&data, &mut bytes);
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
    }
}


pub fn decode(bytes: &[u8]) -> Result<Data, String> {
    if bytes.len() < 4 || &bytes[0..3] != b"GLH" {
        return Err("invalid header: expected GLH magic bytes".to_string());
    }
    let mut pos = 4;
    decode_value(bytes, &mut pos)
}


fn decode_value(bytes: &[u8], pos: &mut usize) -> Result<Data, String> {
    let tag = *bytes.get(*pos).ok_or("unexpected end of data: missing tag")?;
    *pos += 1;

    match tag {
        0 => Ok(Data::Null),

        1 => {
            let value = read_i64(bytes, pos)?;
            Ok(Data::Int(value))
        }

        2 => {
            let byte = *bytes.get(*pos).ok_or("unexpected eof: missing bool")?;
            *pos += 1;
            Ok(Data::Bool(byte != 0))
        }

        3 => {
            let value = read_f64(bytes, pos)?;
            Ok(Data::Float(value))
        }

        4 => {
            let length = read_u32(bytes, pos)? as usize;
            let slice = read_slice(bytes, pos, length)?;
            let string = String::from_utf8(slice.to_vec())
                .map_err(|_| "invalid utf8 in string".to_string())?;
            Ok(Data::String(string))
        }

        5 => {
            let length = read_u32(bytes, pos)? as usize;
            let mut items = Vec::with_capacity(length);
            for _ in 0..length {
                items.push(decode_value(bytes, pos)?);
            }
            Ok(Data::Array(items))
        }

        6 => {
            let length = read_u32(bytes, pos)? as usize;
            let slice = read_slice(bytes, pos, length)?;
            Ok(Data::Bytes(slice.to_vec()))
        }

        7 => {
            let length = read_u32(bytes, pos)? as usize;
            let mut entries = Vec::with_capacity(length);
            for _ in 0..length {
                let key_length = read_u32(bytes, pos)? as usize;
                let key_slice = read_slice(bytes, pos, key_length)?;
                let key = String::from_utf8(key_slice.to_vec())
                    .map_err(|_| "invalid utf8 in object key".to_string())?;
                let value = decode_value(bytes, pos)?;
                entries.push((key, value));
            }
            Ok(Data::Object(entries))
        }

        other => Err(format!("unknown type tag: {}", other)),
    }
}



fn read_slice<'a>(bytes: &'a [u8], pos: &mut usize, length: usize) -> Result<&'a [u8], String> {
    let end = pos.checked_add(length).ok_or("length overflow")?;
    let slice = bytes.get(*pos..end).ok_or("unexpected eof: slice out of range")?;
    *pos = end;
    Ok(slice)
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    let slice = read_slice(bytes, pos, 4)?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

fn read_i64(bytes: &[u8], pos: &mut usize) -> Result<i64, String> {
    let slice = read_slice(bytes, pos, 8)?;
    Ok(i64::from_le_bytes(slice.try_into().unwrap()))
}

fn read_f64(bytes: &[u8], pos: &mut usize) -> Result<f64, String> {
    let slice = read_slice(bytes, pos, 8)?;
    Ok(f64::from_le_bytes(slice.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_null() {
        let bytes = encode(Data::Null);
        assert_eq!(decode(&bytes).unwrap(), Data::Null);
    }

    #[test]
    fn round_trip_int() {
        let bytes = encode(Data::Int(-42));
        assert_eq!(decode(&bytes).unwrap(), Data::Int(-42));
    }

    #[test]
    fn round_trip_bool() {
        let bytes = encode(Data::Bool(true));
        assert_eq!(decode(&bytes).unwrap(), Data::Bool(true));
    }

    #[test]
    fn round_trip_float() {
        let bytes = encode(Data::Float(3.14));
        assert_eq!(decode(&bytes).unwrap(), Data::Float(3.14));
    }

    #[test]
    fn round_trip_string() {
        let bytes = encode(Data::String("Jabbo".to_string()));
        assert_eq!(decode(&bytes).unwrap(), Data::String("Jabbo".to_string()));
    }

    #[test]
    fn round_trip_bytes() {
        let bytes = encode(Data::Bytes(vec![1, 2, 3, 255]));
        assert_eq!(decode(&bytes).unwrap(), Data::Bytes(vec![1, 2, 3, 255]));
    }

    #[test]
    fn round_trip_nested_array() {
        let data = Data::Array(vec![
            Data::Int(1),
            Data::String("two".to_string()),
            Data::Array(vec![Data::Bool(false), Data::Null]),
        ]);
        let bytes = encode(data);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(
            decoded,
            Data::Array(vec![
                Data::Int(1),
                Data::String("two".to_string()),
                Data::Array(vec![Data::Bool(false), Data::Null]),
            ])
        );
    }

    #[test]
    fn round_trip_object() {
        let data = Data::Object(vec![
            ("name".to_string(), Data::String("Jabbo".to_string())),
            ("age".to_string(), Data::Int(5)),
            (
                "tags".to_string(),
                Data::Array(vec![Data::String("a".to_string())]),
            ),
        ]);
        let bytes = encode(data);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(
            decoded,
            Data::Object(vec![
                ("name".to_string(), Data::String("Jabbo".to_string())),
                ("age".to_string(), Data::Int(5)),
                (
                    "tags".to_string(),
                    Data::Array(vec![Data::String("a".to_string())])
                ),
            ])
        );
    }

    #[test]
    fn decode_rejects_bad_header() {
        let bad = vec![b'X', b'X', b'X', 1, 0];
        assert!(decode(&bad).is_err());
    }

    #[test]
    fn decode_rejects_truncated_data() {
        let mut bytes = encode(Data::String("hello".to_string()));
        bytes.truncate(bytes.len() - 3);
        assert!(decode(&bytes).is_err());
    }

    // --- Новые тесты на From-конверсии ---

    #[test]
    fn from_str_literal() {
        // теперь можно закинуть просто "hello", без .to_string() и без Data::String
        let bytes = serialize_to_bytes("hello");
        assert_eq!(decode(&bytes).unwrap(), Data::String("hello".to_string()));
    }

    #[test]
    fn from_i32() {
        let bytes = serialize_to_bytes(42);
        assert_eq!(decode(&bytes).unwrap(), Data::Int(42));
    }

    #[test]
    fn from_vec_of_strings() {
        // Vec<&str> -> Data::Array(Vec<Data::String>) благодаря generic-impl From<Vec<T>>
        let bytes = serialize_to_bytes(vec!["a", "b", "c"]);
        assert_eq!(
            decode(&bytes).unwrap(),
            Data::Array(vec![
                Data::String("a".to_string()),
                Data::String("b".to_string()),
                Data::String("c".to_string()),
            ])
        );
    }
}