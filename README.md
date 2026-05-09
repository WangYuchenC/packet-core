# packet-core

Core library for packet definition parsing and codec.

## Overview

`packet-core` provides a complete solution for defining, parsing, validating, encoding, and decoding packet structures. It is the foundational crate for the Packet project.

## Features

- **Parser**: Complete DSL parsing for `.pkt` files using logos and chumsky
- **AST**: Rich abstract syntax tree types for schema definitions
- **Validator**: Comprehensive schema validation with error codes (E001-E015)
- **Codec**: High-performance encoding and decoding with configurable byte order
- **Type Safety**: Strongly typed with comprehensive error handling and precise error locations

## Supported Types

### Primitive Types
- Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`
- Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`
- Floating point: `f32`, `f64`
- Boolean: `bool`

### Complex Types
- Strings: `String`
- Byte arrays: `Bytes`
- Fixed arrays: `[T; N]`
- Vectors: `Vec<T>`
- Custom types: User-defined structs

## Quick Start

```rust
use packet_core::{parse_schema, validate_schema, Codec};

let schema = parse_schema(r#"
    #![version("1.0.0")]
    #![endian(big)]

    #[send]
    struct MyPacket {
        id: u32 = 0x12345678,
        value: u16 = 42,
    }
"#)?;

validate_schema(&schema)?;

let codec = Codec::compile(&schema, "MyPacket")?;
let bytes = codec.encode()?;
```

## Field Attributes

| Attribute | Syntax | Description |
|-----------|--------|-------------|
| `auto` | `#[auto]` / `#[auto(field)]` | Auto-calculate total length or field length |
| `len_ref` | `#[len_ref(field)]` | Length reference for variable data (receive only) |
| `remaining` | `#[remaining]` | Capture all remaining bytes (receive only) |
| `if` | `#[if(condition)]` | Conditional field |
| `bits` | `#[bits(start, end)]` | Multi-bit field |
| `bit` | `#[bit(position)]` | Single-bit field |
| `endian` | `#[endian(big)]` / `#[endian(little)]` | Field-level byte order |
| `checksum` | `#[checksum(algo)]` | Checksum calculation |

## Error Codes

| Code | Description |
|------|-------------|
| E001 | Struct not found |
| E002 | Duplicate struct name |
| E003 | Duplicate field name |
| E004 | Send struct field missing value |
| E005 | Empty field name |
| E006 | Invalid if condition field |
| E007 | Invalid len_ref field |
| E008 | Checksum field must be unsigned integer |
| E009 | Unknown type |
| E010 | Zero-size array |
| E011-E015 | Remaining field constraints |

## Related Projects

- [packet-transport](https://github.com/WangYuchenC/packet-transport) - Transport layer abstraction and plugin registry
- [packet-cli](https://github.com/WangYuchenC/packet-cli) - CLI tool of packet project

## License

MIT OR Apache-2.0
