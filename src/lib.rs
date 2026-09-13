use std::fs::File;
use std::io::{Read as IoRead, Write};
use std::path::{Path, PathBuf};

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

pub fn write(data: Vec<u8>, folder: PathBuf, file_name: String) {
    let path = folder.join(file_name);
    let mut file = File::create(path).unwrap();
    file.write_all(&data).unwrap();
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
    // bytes[3] — версия формата. Пока не используется,
    // но в будущем сюда можно завести ветвление по версии.
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

// --- маленькие хелперы для чтения примитивов из буфера ---

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

pub fn read(path: &Path) -> Result<Data, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    decode(&buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_write_complex() {
    //     let data = Data::Object(vec![
    //         ("id".to_string(), Data::Int(-9001)),
    //         ("name".to_string(), Data::String("Jabbo the Glyph".to_string())),
    //         ("active".to_string(), Data::Bool(true)),
    //         ("score".to_string(), Data::Float(3.14159)),
    //         ("deleted_at".to_string(), Data::Null),
    //         (
    //             "thumbnail".to_string(),
    //             Data::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF]),
    //         ),
    //         (
    //             "tags".to_string(),
    //             Data::Array(vec![
    //                 Data::String("worldbuilding".to_string()),
    //                 Data::String("rust".to_string()),
    //                 Data::String("tauri".to_string()),
    //             ]),
    //         ),
    //         (
    //             "stats".to_string(),
    //             Data::Object(vec![
    //                 ("views".to_string(), Data::Int(1200)),
    //                 ("rating".to_string(), Data::Float(4.7)),
    //                 (
    //                     "history".to_string(),
    //                     Data::Array(vec![
    //                         Data::Object(vec![
    //                             ("date".to_string(), Data::String("2026-01-01".to_string())),
    //                             ("value".to_string(), Data::Int(10)),
    //                         ]),
    //                         Data::Object(vec![
    //                             ("date".to_string(), Data::String("2026-02-01".to_string())),
    //                             ("value".to_string(), Data::Int(25)),
    //                         ]),
    //                     ]),
    //                 ),
    //             ]),
    //         ),
    //         (
    //             "empty_array".to_string(),
    //             Data::Array(vec![]),
    //         ),
    //         (
    //             "empty_object".to_string(),
    //             Data::Object(vec![]),
    //         ),
    //         (
    //             "nested_nulls".to_string(),
    //             Data::Array(vec![Data::Null, Data::Null, Data::Bool(false)]),
    //         ),
    //     ]);
    //
    //     // Кодируем и пишем на диск — как в test_write
    //     let bytes = encode(data);
    //     write(bytes.clone());
    //
    //     // И сразу проверяем, что round-trip (encode -> decode) не теряет данные
    //     let decoded = decode(&bytes).unwrap();
    //
    //     match decoded {
    //         Data::Object(fields) => {
    //             assert_eq!(fields.len(), 11);
    //             assert_eq!(fields[0], ("id".to_string(), Data::Int(-9001)));
    //             assert_eq!(
    //                 fields[1],
    //                 ("name".to_string(), Data::String("Jabbo the Glyph".to_string()))
    //             );
    //         }
    //         _ => panic!("expected top-level Object"),
    //     }
    // }

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
            ("tags".to_string(), Data::Array(vec![Data::String("a".to_string())])),
        ]);
        let bytes = encode(data);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(
            decoded,
            Data::Object(vec![
                ("name".to_string(), Data::String("Jabbo".to_string())),
                ("age".to_string(), Data::Int(5)),
                ("tags".to_string(), Data::Array(vec![Data::String("a".to_string())])),
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
        bytes.truncate(bytes.len() - 3); // обрезаем конец строки
        assert!(decode(&bytes).is_err());
    }
}