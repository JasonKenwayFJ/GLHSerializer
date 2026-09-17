# GLHSerializer

A small, self-contained binary serialization format for structured data, written in Rust.

GLH encodes a tagged value tree — nulls, bools, ints, floats, strings, raw bytes, arrays,
ordered objects and hash maps — into a compact byte buffer with an 8-byte header. Originally
built for the [Glyph](https://github.com/JasonKenwayFJ/Glyph) project, but it has no dependency on it.

```toml
[dependencies]
glhserializer = "0.5"
```

> The crate is published as `glhserializer`, but the library target is named `glh`,
> so in code you import it as `glh::...`.

---

## Quick start

### Work with the value tree directly

```rust
use glh::models::data::Data;
use glh::functions::writer::encode;
use glh::functions::reader::decode;

let data = Data::Object(vec![
    ("name".to_string(), Data::String("Jabbo".to_string())),
    ("age".to_string(),  Data::Int(5)),
]);

let bytes = encode(data);                 // Vec<u8>
let decoded = decode(&bytes).unwrap();    // Data
```

`decode` returns `Result<Data, String>` — it validates the header and never panics on
malformed input.

### Work with your own types

Anything that implements `serde::Serialize` / `Deserialize` can be written and read back:

```rust
use std::path::{Path, PathBuf};
use glh::functions::writer::write_typed;
use glh::functions::reader::read;

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
struct Character {
    name: String,
    level: i64,
    is_alive: bool,
    tags: Vec<String>,
}

let hero = Character {
    name: "Arthur".into(),
    level: 12,
    is_alive: true,
    tags: vec!["knight".into(), "leader".into()],
};

write_typed(PathBuf::from("./saves"), "hero".to_string(), &hero)?;
let restored: Character = read(Path::new("./saves/hero.glh"))?;

assert_eq!(hero, restored);
```

Under the hood this bridges through `serde_json::Value`: your type is serialized to JSON,
the JSON tree is converted to `Data`, and `Data` is encoded to GLH bytes. Reading goes the
other way round.

> `write` and `write_typed` both append `.glh` to `file_name` automatically if it doesn't
> already have an extension — pass a bare name like `"hero"`, or a full file name like
> `"hero.glh"`, either works. An extension other than `.glh` (e.g. `"save.bak"`) is left
> alone.

### Convert from primitives

`Data` implements `From` for the common Rust types, so `serialize_to_bytes` accepts them
directly:

```rust
use glh::functions::writer::serialize_to_bytes;

let bytes = serialize_to_bytes("hello");          // Data::String
let bytes = serialize_to_bytes(42);               // Data::Int
let bytes = serialize_to_bytes(vec!["a", "b"]);   // Data::Array of Data::String
```

Implemented conversions: `String`, `&str`, `bool`, `i32`, `i64`, `f32`, `f64`, `Vec<u8>`,
`HashMap<String, Data>`, and `Vec<T>` for any `T: Into<Data>`.

---

## API

### `glh::models::data`

| Item | Description |
| --- | --- |
| `enum Data` | The value tree: `Null`, `Bool`, `Int(i64)`, `Float(f64)`, `String`, `Bytes(Vec<u8>)`, `Array(Vec<Data>)`, `Object(Vec<(String, Data)>)`, `Hash(HashMap<String, Data>)` |

`Object` preserves key order (it is a `Vec` of pairs); `Hash` does not. Pick whichever
matches the semantics you need.

### `glh::functions::writer`

| Function | Description |
| --- | --- |
| `encode(data: Data) -> Vec<u8>` | Encode a value tree, header included |
| `serialize_to_bytes<T: Into<Data>>(data: T) -> Vec<u8>` | Convert and encode in one step |
| `write<T: Into<Data>>(folder, file_name, data) -> Result<(), String>` | Encode and write to disk |
| `write_typed<T: Serialize>(folder, file_name, &data) -> Result<(), String>` | Write any serde type to disk |

### `glh::functions::reader`

| Function | Description |
| --- | --- |
| `decode(bytes: &[u8]) -> Result<Data, String>` | Verify the header, then decode the tree |
| `decode_value(bytes: &[u8], pos: &mut usize) -> Result<Data, String>` | Decode a single value at a cursor position |
| `verify(bytes: &[u8]) -> Result<(), String>` | Check magic, version and declared length |
| `read<T: DeserializeOwned>(path: &Path) -> Result<T, String>` | Read a file into your own type |
| `read_raw(path: &Path) -> Result<Data, String>` | Read a file as a raw `Data` tree |
| `data_to_json(data: Data) -> serde_json::Value` | Convert a `Data` tree to JSON |

### `glh::helpers`

Little-endian cursor readers used by the decoder, exposed in case you want to build your
own: `read_slice`, `read_u32`, `read_i64`, `read_f64`. Each takes `pos: &mut usize` and
advances it, so no intermediate slices are allocated while walking a buffer.

### `glh` (crate root)

`serialize_to_json` / `deserialize_from_json` — thin `serde_json` wrappers returning
`Result<_, String>`.

---

## Format specification

### Header — 8 bytes

| Offset | Size | Content |
| --- | --- | --- |
| 0 | 3 | Magic `GLH` (ASCII) |
| 3 | 1 | Format version, currently `1` |
| 4 | 4 | Total file length in bytes, `u32` little-endian |

The length field is patched in after the payload has been written, and `verify` rejects any
buffer whose real length disagrees with it — which catches truncated and padded files before
decoding starts.

### Values

Every value starts with a single tag byte. All multi-byte numbers are little-endian, and all
lengths and counts are `u32`.

| Tag | Type | Payload |
| --- | --- | --- |
| `0` | Null | — |
| `1` | Int | `i64` (8 bytes) |
| `2` | Bool | 1 byte, `0` = false, anything else = true |
| `3` | Float | `f64` (8 bytes) |
| `4` | String | `u32` byte length, then UTF-8 bytes |
| `5` | Array | `u32` element count, then that many encoded values |
| `6` | Bytes | `u32` byte length, then raw bytes |
| `7` | Object | `u32` entry count, then per entry: `u32` key length, UTF-8 key, encoded value |
| `8` | Hash | Same layout as Object; decoded into a `HashMap`, so order is not preserved |

Nesting is recursive: arrays, objects and hashes hold full values, including other
containers, with no depth limit imposed by the format.

### Error handling

The decoder returns `Err(String)` — it never panics — for a buffer shorter than the header,
a bad magic, an unsupported version, a length mismatch, a truncated payload, an unknown type
tag, or invalid UTF-8 in a string or a key.

---

## Notes and caveats

- **`Bytes` do not survive the typed path.** `write_typed` / `read` route through JSON, and
  JSON has no byte-string type, so `Data::Bytes` becomes an array of numbers. Use `encode` /
  `decode` or `read_raw` when you need byte fidelity.
- **Floats that are not finite become `Null` on the typed path.** `data_to_json` drops `NaN`
  and infinities silently, because `serde_json` cannot represent them — a value written as
  `NaN` will read back as whatever your type's default for that field is (or fail to
  deserialize, depending on the type). This only affects `write_typed` / `read`; `encode` /
  `decode` preserve `NaN` and infinities exactly, since floats are stored as raw `f64` bits.
- **`Object` vs `Hash`.** Encoding is identical; only the decoded Rust type differs. Choose
  `Object` if key order carries meaning.
- **File extension.** `write` and `write_typed` add `.glh` automatically if `file_name` has
  no extension already; an existing extension (including a non-`.glh` one) is left as-is.

## Testing

```bash
cargo test
```

The suite covers round-trips for every `Data` variant (including empty collections, integer
and float boundaries, Unicode strings and deeply nested mixtures), rejection of malformed
input, and full write-to-disk-and-read-back cycles for arbitrary serde structs — including a
struct containing a tuple field, to confirm the JSON bridge isn't tied to any particular
shape.

## License

MIT
