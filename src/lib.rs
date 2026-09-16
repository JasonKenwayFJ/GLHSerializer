mod models;
mod functions;
pub mod helpers;

use serde::ser::{self, Serialize, SerializeSeq, SerializeStruct, Serializer};
use serde::Deserialize;
use std::io::{Read as IoRead, Write};
use serde_json::Value as JsonValue;
use crate::models::data::*;

pub fn serialize_to_json<T: Serialize>(data: &T) -> Result<String, String> {
    serde_json::to_string(data).map_err(|e| e.to_string())
}
pub fn deserialize_from_json<'a, T: Deserialize<'a>>(data: &'a str) -> Result<T, String> {
    serde_json::from_str(data).map_err(|e| e.to_string())
}







fn to_data<T: Serialize>(value: &T) -> Result<Data, String> {
    let value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    json_to_data(value)
}
pub fn from_data<T: for<'de> Deserialize<'de>>(
    data: Data
) -> Result<T, String> {
    let value = data_to_json(data)?;

    serde_json::from_value(value)
        .map_err(|e| e.to_string())
}
fn data_to_json(data: Data) -> Result<JsonValue, String> {
    match data {
        Data::Null => {
            Ok(JsonValue::Null)
        }

        Data::Bool(value) => {
            Ok(JsonValue::Bool(value))
        }

        Data::Int(value) => {
            Ok(JsonValue::Number(value.into()))
        }

        Data::Float(value) => {
            let number = serde_json::Number::from_f64(value)
                .ok_or("invalid float")?;

            Ok(JsonValue::Number(number))
        }

        Data::String(value) => {
            Ok(JsonValue::String(value))
        }

        Data::Bytes(value) => {
            Ok(JsonValue::Array(
                value.into_iter()
                    .map(|x| JsonValue::Number(x.into()))
                    .collect()
            ))
        }

        Data::Array(value) => {
            let mut result = Vec::new();

            for item in value {
                result.push(data_to_json(item)?);
            }

            Ok(JsonValue::Array(result))
        }

        Data::Object(value) => {
            let mut result = serde_json::Map::new();

            for (key, value) in value {
                result.insert(
                    key,
                    data_to_json(value)?
                );
            }

            Ok(JsonValue::Object(result))
        }

        Data::Hash(value) => {
            let mut result = serde_json::Map::new();

            for (key, value) in value {
                result.insert(
                    key,
                    data_to_json(value)?
                );
            }

            Ok(JsonValue::Object(result))
        }
    }
}
fn json_to_data(value: JsonValue) -> Result<Data, String> {
    match value {
        JsonValue::Null => Ok(Data::Null),

        JsonValue::Bool(value) => {
            Ok(Data::Bool(value))
        }

        JsonValue::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(Data::Int(value))
            } else if let Some(value) = value.as_f64() {
                Ok(Data::Float(value))
            } else {
                Err("unsupported number".to_string())
            }
        }

        JsonValue::String(value) => {
            Ok(Data::String(value))
        }

        JsonValue::Array(value) => {
            let mut result = Vec::new();

            for item in value {
                result.push(json_to_data(item)?);
            }

            Ok(Data::Array(result))
        }

        JsonValue::Object(value) => {
            let mut result = Vec::new();

            for (key, value) in value {
                result.push((
                    key,
                    json_to_data(value)?
                ));
            }

            Ok(Data::Object(result))
        }
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use crate::functions::reading::decode;
    use crate::functions::writing::{encode, serialize_to_bytes};

    #[test]
    fn test_struct_reader() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Weapon {
            name: String,
            damage: i32,
        }

        let weapon = Weapon {
            name: "Sword".to_string(),
            damage: 10,
        };

        let data = to_data(&weapon).unwrap();

        println!("{data:#?}");

        let restored: Weapon = from_data(data).unwrap();

        assert_eq!(restored, weapon);
    }
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



    #[test]
    fn from_str_literal() {
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
