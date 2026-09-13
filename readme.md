@"
# GLHSerializer

Custom binary serialization format for structured data — nulls, bools, ints, floats, strings, bytes, arrays, and objects.

Originally built for the Glyph project.

## Usage

``````rust
use glhserializer::{Data, encode, decode};

let data = Data::Object(vec![
    ("name".to_string(), Data::String("Jabbo".to_string())),
    ("age".to_string(), Data::Int(5)),
]);

let bytes = encode(data);
let decoded = decode(&bytes).unwrap();
``````

## License

MIT
"@ | Out-File -FilePath README.md -Encoding utf8