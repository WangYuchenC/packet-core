//! # Packet Core
//!
//! Packet 协议解析器和编解码器核心库，用于解析 `.pkt` 文件格式并执行二进制数据的编码/解码。
//!
//! ## 架构概览
//!
//! ```text
//! .pkt 文件
//!   │
//!   ├── Parser ──────> AST (Schema, StructDef, Field)
//!   │
//!   ├── Validator ───> 验证 Schema 完整性
//!   │
//!   ├── Loader ──────> 处理 import 并合并多个文件
//!   │
//!   └── Codec ───────> 二进制编码/解码
//! ```
//!
//! ## 快速开始
//!
//! ```rust
//! use packet_core::{parse_schema, validate_schema, Codec};
//!
//! // 1. 解析 schema
//! let schema = parse_schema(r#"
//!     #![version("1.0.0")]
//!     #![endian(big)]
//!
//!     #[send]
//!     struct MyPacket {
//!         id: u32 = 0x12345678,
//!         value: u16 = 42,
//!     }
//! "#)?;
//!
//! // 2. 验证 schema
//! validate_schema(&schema)?;
//!
//! // 3. 编译编解码器
//! let codec = Codec::compile(&schema, "MyPacket")?;
//!
//! // 4. 编码
//! let bytes = codec.encode()?;
//! # Ok::<(), packet_core::CoreError>(())
//! ```
//!
//! ## 支持的数据类型
//!
//! | 类型 | 大小 | 说明 |
//! |------|------|------|
//! | `u8`/`u16`/`u32`/`u64`/`u128` | 1-16 字节 | 无符号整数 |
//! | `i8`/`i16`/`i32`/`i64`/`i128` | 1-16 字节 | 有符号整数 |
//! | `f32`/`f64` | 4/8 字节 | IEEE 754 浮点数 |
//! | `bool` | 1 字节 | 布尔值 |
//! | `String` | 变长 | UTF-8 字符串 |
//! | `Bytes` | 固定长度 | 原始字节 |
//! | `Vec<T>` | 变长 | 动态数组 |
//! | `[T; N]` | 固定长度 | 静态数组 |
//!
//! ## 属性支持
//!
//! ### 结构体属性
//!
//! | 属性 | 说明 |
//! |------|------|
//! | `#[send]` | 发送类型（编码用） |
//! | `#[receive]` | 接收类型（解码用） |
//!
//! ### 字段属性
//!
//! | 属性 | 适用对象 | 说明 |
//! |------|---------|------|
//! | `#[auto]` | 数值字段 | 自动计算后续字段总长度 |
//! | `#[len_ref = field]` | 数值字段 | 引用指定 `Vec` 字段的长度 |
//! | `#[remaining]` | `Bytes` 或 `Vec<u8>` | 捕获数据包剩余字节 |
//! | `#[endian = little/big]` | 数值字段 | 指定字节序 |
//! | `#[checksum = algo]` | 数值字段 | 计算前面所有字段的校验和 |
//! | `#[bits(start, end)]` | 数值字段 | 位域（起始位-结束位） |
//! | `#[if = condition]` | 任意字段 | 条件字段（依赖 bool 字段） |
//!
//! ### 校验和算法
//!
//! | 算法 | 说明 |
//! |------|------|
//! | `crc8` | CRC-8 |
//! | `crc16` | CRC-16 |
//! | `crc32` | CRC-32 |
//! | `xor` | 异或校验 |
//! | `sum` | 求和校验 |
//!
//! ## 文件级属性
//!
//! ```text
//! #![version = "x.y.z"]   // 版本声明
//! #![endian = big/little] // 文件默认字节序（默认 big）
//! ```
//!
//! ## Import 支持
//!
//! ```text
//! import "common/types.pkt"
//! ```
//!
//! 使用 [`load_schema`](loader::load_schema) 加载文件时，
//! 会自动递归处理 `import` 语句并合并所有 schema。
//!
//! ## 错误处理
//!
//! 所有 API 使用 [`CoreError`] 作为统一错误类型。
//! 使用 [`Result<T>`](Result) 作为返回类型。

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

pub mod ast;
pub mod codec;
pub mod error;
pub mod loader;
pub mod parser;
pub mod validator;

// Re-exports
pub use codec::{Codec, DecodedStruct, DecodedValue};
pub use error::{CodecErrorKind, CoreError, ParseErrorKind, Result, ValidationFailureKind};
pub use parser::parse_schema;
pub use validator::validate_schema;

/// 主要文件扩展名
pub const FILE_EXTENSION: &str = "pkt";

/// 替代文件扩展名
pub const FILE_EXTENSION_ALT: &str = "packet";

/// 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
