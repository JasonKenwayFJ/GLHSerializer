use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use serde::de::DeserializeOwned;
use crate::models::data::Data;
use crate::helpers;




pub fn read<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    let decoded_data = decode(&buffer)?;
    let json_value = data_to_json(decoded_data);
    serde_json::from_value(json_value).map_err(|e| e.to_string())
}
pub fn read_raw(path: &Path) -> Result<Data, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    decode(&buffer)
}
pub fn data_to_json(data: Data) -> serde_json::Value {
    match data {
        Data::Null => serde_json::Value::Null,
        Data::Bool(b) => serde_json::Value::Bool(b),
        Data::Int(i) => serde_json::Value::Number(i.into()),
        Data::Float(f) => serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Data::String(s) => serde_json::Value::String(s),
        Data::Bytes(b) => serde_json::Value::Array(
            b.into_iter().map(|byte| serde_json::Value::Number(byte.into())).collect()
        ),
        Data::Array(arr) => serde_json::Value::Array(
            arr.into_iter().map(data_to_json).collect()
        ),
        Data::Object(entries) => serde_json::Value::Object(
            entries.into_iter().map(|(k, v)| (k, data_to_json(v))).collect()
        ),
        Data::Hash(map) => serde_json::Value::Object(
            map.into_iter().map(|(k, v)| (k, data_to_json(v))).collect()
        ),
    }
}

pub fn verify(bytes: &[u8]) -> Result<(), String> {
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
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use crate::functions::writer::{encode, write_typed};

    // Уникальный путь во временной папке ОС — чтобы параллельные тесты
    // не затирали файлы друг друга (у cargo test каждый тест — своя нить).
    fn temp_path(label: &str) -> PathBuf {
        let unique = format!(
            "{}_{}_{}.glh",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    // ---------- Round-trip для каждого варианта Data ----------

    #[test]
    fn round_trip_null() {
        let bytes = encode(Data::Null);
        assert_eq!(decode(&bytes).unwrap(), Data::Null);
    }

    #[test]
    fn round_trip_bool_true_and_false() {
        assert_eq!(decode(&encode(Data::Bool(true))).unwrap(), Data::Bool(true));
        assert_eq!(decode(&encode(Data::Bool(false))).unwrap(), Data::Bool(false));
    }

    #[test]
    fn round_trip_int_boundaries() {
        for value in [0_i64, -1, 1, i64::MAX, i64::MIN] {
            assert_eq!(decode(&encode(Data::Int(value))).unwrap(), Data::Int(value));
        }
    }

    #[test]
    fn round_trip_float_boundaries() {
        for value in [0.0_f64, -0.0, 3.14, -3.14, f64::MAX, f64::MIN] {
            assert_eq!(decode(&encode(Data::Float(value))).unwrap(), Data::Float(value));
        }
    }

    #[test]
    fn round_trip_string_empty_and_unicode() {
        for value in ["", "hello", "Артур", "🗡️", "a".repeat(10_000).as_str()] {
            let data = Data::String(value.to_string());
            assert_eq!(decode(&encode(data)).unwrap(), Data::String(value.to_string()));
        }
    }

    #[test]
    fn round_trip_bytes_empty_and_filled() {
        for value in [vec![], vec![0, 1, 2, 255], vec![7; 5000]] {
            let data = Data::Bytes(value.clone());
            assert_eq!(decode(&encode(data)).unwrap(), Data::Bytes(value));
        }
    }

    #[test]
    fn round_trip_array_empty_and_nested() {
        let empty = Data::Array(vec![]);
        assert_eq!(decode(&encode(empty)).unwrap(), Data::Array(vec![]));

        let nested = Data::Array(vec![
            Data::Int(1),
            Data::Array(vec![Data::Bool(true), Data::Null]),
            Data::String("глубоко".into()),
        ]);
        let bytes = encode(nested.clone());
        assert_eq!(decode(&bytes).unwrap(), nested);
    }

    #[test]
    fn round_trip_object_empty_and_filled() {
        let empty = Data::Object(vec![]);
        assert_eq!(decode(&encode(empty)).unwrap(), Data::Object(vec![]));

        let filled = Data::Object(vec![
            ("name".into(), Data::String("Артур".into())),
            ("level".into(), Data::Int(12)),
        ]);
        assert_eq!(decode(&encode(filled.clone())).unwrap(), filled);
    }

    #[test]
    fn round_trip_hash_empty_and_filled() {
        let mut map = HashMap::new();
        map.insert("a".to_string(), Data::Int(1));
        map.insert("b".to_string(), Data::Bool(false));
        let data = Data::Hash(map.clone());
        match decode(&encode(data)).unwrap() {
            Data::Hash(decoded) => assert_eq!(decoded, map),
            other => panic!("ожидался Data::Hash, получено {other:?}"),
        }
    }

    #[test]
    fn round_trip_deeply_nested_mixed() {
        let data = Data::Object(vec![(
            "root".into(),
            Data::Array(vec![Data::Object(vec![(
                "child".into(),
                Data::Array(vec![Data::Int(1), Data::Int(2), Data::Int(3)]),
            )])]),
        )]);
        assert_eq!(decode(&encode(data.clone())).unwrap(), data);
    }

    // ---------- Некорректные / повреждённые данные ----------

    #[test]
    fn verify_rejects_too_short_input() {
        assert!(verify(&[b'G', b'L', b'H']).is_err()); // меньше 8 байт
    }

    #[test]
    fn verify_rejects_bad_magic() {
        let bad = [b'X', b'X', b'X', 1, 0, 0, 0, 8];
        assert!(verify(&bad).is_err());
    }

    #[test]
    fn verify_rejects_unsupported_version() {
        let bad = [b'G', b'L', b'H', 99, 0, 0, 0, 8];
        assert!(verify(&bad).is_err());
    }

    #[test]
    fn verify_rejects_size_mismatch() {
        // Заголовок утверждает 100 байт, а по факту файл — 8 байт
        let bad = [b'G', b'L', b'H', 1, 100, 0, 0, 0];
        assert!(verify(&bad).is_err());
    }

    #[test]
    fn decode_rejects_truncated_payload() {
        let mut bytes = encode(Data::String("hello".to_string()));
        bytes.truncate(bytes.len() - 3);
        assert!(decode(&bytes).is_err());
    }

    #[test]
    fn decode_rejects_unknown_type_tag() {
        // Валидный заголовок, но byte-тег 255 не существует ни для одного варианта Data
        let mut bytes = encode(Data::Null);
        let tag_pos = bytes.len() - 1; // последний байт — тег Null (0)
        bytes[tag_pos] = 255;
        assert!(decode(&bytes).is_err());
    }

    #[test]
    fn decode_rejects_invalid_utf8_in_string() {
        let mut bytes = encode(Data::String("ok".to_string()));
        let len = bytes.len();
        bytes[len - 1] = 0xFF; // портим последний байт содержимого строки
        assert!(decode(&bytes).is_err());
    }

    // ---------- Твой сценарий: трейт → объект → файл → чтение → новый объект ----------

    trait Describable {
        fn describe(&self) -> String;
    }

    #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
    struct Character {
        name: String,
        level: i64,
        is_alive: bool,
        tags: Vec<String>,
    }

    impl Describable for Character {
        fn describe(&self) -> String {
            format!("{} (уровень {})", self.name, self.level)
        }
    }

    #[test]
    fn serializer_adapts_to_arbitrary_struct_via_trait() {
        let original = Character {
            name: "Артур".to_string(),
            level: 12,
            is_alive: true,
            tags: vec!["рыцарь".to_string(), "лидер".to_string()],
        };

        let path = temp_path("character");
        let folder = path.parent().unwrap().to_path_buf();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        write_typed(folder, file_name, &original).expect("запись не удалась");

        let restored: Character = read(&path).expect("чтение не удалось");

        // Данные совпадают полностью
        assert_eq!(original, restored);
        // Поведение через трейт тоже совпадает — не только поля, но и логика типа сохранились
        assert_eq!(original.describe(), restored.describe());

        std::fs::remove_file(&path).ok(); // подчистили за собой
    }

    #[test]
    fn serializer_adapts_to_different_struct_shape() {
        // Другая форма структуры — проверяем, что мост Data <-> JSON
        // не завязан жёстко на конкретные поля, а работает для любой T: Serialize/Deserialize
        #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct Location {
            title: String,
            coordinates: (f64, f64),
            visited: bool,
        }

        let original = Location {
            title: "Эльдория".to_string(),
            coordinates: (12.5, -3.25),
            visited: false,
        };

        let path = temp_path("location");
        let folder = path.parent().unwrap().to_path_buf();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        write_typed(folder, file_name, &original).unwrap();
        let restored: Location = read(&path).unwrap();

        assert_eq!(original, restored);
        std::fs::remove_file(&path).ok();
    }
}