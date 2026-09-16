use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use crate::models::data::Data;
use crate::helpers;
pub fn read(path: &Path) -> Result<Data, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    decode(&buffer)
}

fn verify(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 8 {
        return Err("File is too small".into());
    }

    if &bytes[0..3] != b"GLH" {
        return Err("Invalid GLH header".into());
    }

    if bytes[3] != 1 {
        return Err("Unsupported GLH version".into());
    }

    let expected_size = u32::from_le_bytes(
        bytes[4..8].try_into().unwrap()
    ) as usize;

    if bytes.len() != expected_size {
        return Err("File size does not match header".into());
    }

    Ok(())
}

fn deserialize_from_bytes(data: &[u8]) -> Result<Data, String> {
    decode(data)
}

pub fn decode(bytes: &[u8]) -> Result<Data, String> {
    verify(bytes)?;
    let mut pos = 8;
    decode_value(bytes, &mut pos)
}

pub fn decode_value(bytes: &[u8], pos: &mut usize) -> Result<Data, String> {
    let tag = *bytes
        .get(*pos)
        .ok_or("unexpected end of data: missing tag")?;
    *pos += 1;

    match tag {
        0 => Ok(Data::Null),

        1 => {
            let value = helpers::read_i64(bytes, pos)?;
            Ok(Data::Int(value))
        }

        2 => {
            let byte = *bytes.get(*pos).ok_or("unexpected eof: missing bool")?;
            *pos += 1;
            Ok(Data::Bool(byte != 0))
        }

        3 => {
            let value = helpers::read_f64(bytes, pos)?;
            Ok(Data::Float(value))
        }

        4 => {
            let length = helpers::read_u32(bytes, pos)? as usize;
            let slice = helpers::read_slice(bytes, pos, length)?;
            let string = String::from_utf8(slice.to_vec())
                .map_err(|_| "invalid utf8 in string".to_string())?;
            Ok(Data::String(string))
        }

        5 => {
            let length = helpers::read_u32(bytes, pos)? as usize;
            let mut items = Vec::with_capacity(length);
            for _ in 0..length {
                items.push(decode_value(bytes, pos)?);
            }
            Ok(Data::Array(items))
        }

        6 => {
            let length = helpers::read_u32(bytes, pos)? as usize;
            let slice = helpers::read_slice(bytes, pos, length)?;
            Ok(Data::Bytes(slice.to_vec()))
        }

        7 => {
            let length = helpers::read_u32(bytes, pos)? as usize;
            let mut entries = Vec::with_capacity(length);
            for _ in 0..length {
                let key_length = helpers::read_u32(bytes, pos)? as usize;
                let key_slice = helpers::read_slice(bytes, pos, key_length)?;
                let key = String::from_utf8(key_slice.to_vec())
                    .map_err(|_| "invalid utf8 in object key".to_string())?;
                let value = decode_value(bytes, pos)?;
                entries.push((key, value));
            }
            Ok(Data::Object(entries))
        }
        8 => {
            let length = helpers::read_u32(bytes, pos)? as usize;
            let mut map = HashMap::with_capacity(length);

            for _ in 0..length {
                let key_length = helpers::read_u32(bytes, pos)? as usize;
                let key_slice = helpers::read_slice(bytes, pos, key_length)?;

                let key = String::from_utf8(key_slice.to_vec())
                    .map_err(|_| "invalid utf8 in hash key".to_string())?;

                let value = decode_value(bytes, pos)?;
                map.insert(key, value);
            }

            Ok(Data::Hash(map))
        }
        other => Err(format!("unknown type tag: {}", other)),
    }
}