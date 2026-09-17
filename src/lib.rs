pub mod models;
pub mod functions;
pub mod helpers;

use serde::ser::{Serialize};
use serde::Deserialize;

pub fn serialize_to_json<T: Serialize>(data: &T) -> Result<String, String> {
    serde_json::to_string(data).map_err(|e| e.to_string())
}
pub fn deserialize_from_json<'a, T: Deserialize<'a>>(data: &'a str) -> Result<T, String> {
    serde_json::from_str(data).map_err(|e| e.to_string())
}





#[cfg(test)]
mod tests {

    use crate::functions::reader::decode;
    use crate::functions::writer::{encode, serialize_to_bytes};
    use crate::models::data::Data;

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
