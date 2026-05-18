//! Schema AST 定义

use super::types::{Type, Value};
use std::collections::HashMap;

/// Schema 文件根
#[derive(Debug, Clone)]
pub struct Schema {
    /// 文件级属性列表
    pub file_attributes: Vec<FileAttribute>,
    /// 导入语句列表
    pub imports: Vec<Import>,
    /// 结构体定义列表
    pub structs: Vec<StructDef>,
    /// 名称到索引的映射
    pub struct_map: HashMap<String, usize>,
}

impl Schema {
    /// 创建新的 Schema
    #[must_use]
    pub fn new() -> Self {
        Self {
            file_attributes: Vec::new(),
            imports: Vec::new(),
            structs: Vec::new(),
            struct_map: HashMap::new(),
        }
    }

    /// 添加文件级属性
    pub fn add_file_attribute(&mut self, attr: FileAttribute) {
        self.file_attributes.push(attr);
    }

    /// 添加导入语句
    pub fn add_import(&mut self, import: Import) {
        self.imports.push(import);
    }

    /// 添加结构体定义
    pub fn add_struct(&mut self, def: StructDef) {
        let name = def.name.clone();
        let index = self.structs.len();
        self.structs.push(def);
        self.struct_map.insert(name, index);
    }

    /// 根据名称获取结构体
    #[must_use]
    pub fn get_struct(&self, name: &str) -> Option<&StructDef> {
        self.struct_map
            .get(name)
            .and_then(|&idx| self.structs.get(idx))
    }

    /// 获取第一个 receive 结构体
    #[must_use]
    pub fn get_first_receive_struct(&self) -> Option<&StructDef> {
        self.structs
            .iter()
            .find(|s| s.direction == Some(Direction::Receive))
    }

    /// 获取第一个 send 结构体
    #[must_use]
    pub fn get_first_send_struct(&self) -> Option<&StructDef> {
        self.structs
            .iter()
            .find(|s| s.direction == Some(Direction::Send))
    }

    /// 从文件属性获取默认字节序
    #[must_use]
    pub fn default_endian(&self) -> Endianness {
        self.file_attributes
            .iter()
            .find_map(|attr| match attr {
                FileAttribute::Endian(endian) => Some(*endian),
                _ => None,
            })
            .unwrap_or(Endianness::Big)
    }

    /// 检查是否启用了自动长度前缀
    #[must_use]
    pub fn is_prefix_enabled(&self) -> bool {
        !self
            .file_attributes
            .iter()
            .any(|attr| matches!(attr, FileAttribute::PrefixDisabled))
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

/// 文件级属性
#[derive(Debug, Clone, PartialEq)]
pub enum FileAttribute {
    /// 版本声明 #![version("x.y.z")]
    Version(String),
    /// 字节序声明 #![endian(big/little)]
    Endian(Endianness),
    /// 导入路径前缀 #![`import_path("lib`/")]
    ImportPath(String),
    /// 文件级文档 #![doc("...")]
    Doc(String),
    /// 禁用自动长度前缀 #![prefix(disable)]
    PrefixDisabled,
}

/// 导入语句
#[derive(Debug, Clone)]
pub struct Import {
    /// 导入的文件路径
    pub path: String,
    /// 文档注释
    pub doc: Option<String>,
}

impl Import {
    /// 创建新的导入语句
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            doc: None,
        }
    }
}

/// 结构体定义
#[derive(Debug, Clone)]
pub struct StructDef {
    /// 结构体名称
    pub name: String,
    /// 方向（send/receive）
    pub direction: Option<Direction>,
    /// 字段列表
    pub fields: Vec<Field>,
    /// 文档注释
    pub doc: Option<String>,
    /// 结构体级字节序（覆盖文件级默认值）
    pub endian: Option<Endianness>,
}

/// 方向枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 发送结构体
    Send,
    /// 接收结构体
    Receive,
}

impl Direction {
    /// 转换为字符串
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::Send => "send",
            Direction::Receive => "receive",
        }
    }
}

/// 字段定义
#[derive(Debug, Clone)]
pub struct Field {
    /// 字段名称
    pub name: String,
    /// 字段类型
    pub ty: Type,
    /// 字段值（send 结构体使用）
    pub value: Option<Value>,
    /// 字段属性
    pub attributes: Vec<FieldAttribute>,
    /// 文档注释
    pub doc: Option<String>,
}

/// 字段属性
#[derive(Debug, Clone, PartialEq)]
pub enum FieldAttribute {
    /// 自动计算
    /// - None: 计算整个结构体总字节数
    /// - Some(field): 计算指定字段的元素个数或字节数
    Auto(Option<String>),
    /// 长度引用
    LenRef(String),
    /// 剩余字节
    Remaining,
    /// 位域（多位）
    Bits(usize, usize),
    /// 位域（单比特）
    Bit(usize),
    /// 条件字段
    If(String),
    /// 校验和算法
    Checksum(ChecksumAlgorithm),
    /// 字节序
    Endian(Endianness),
}

/// 校验和算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    /// CRC-8
    Crc8,
    /// CRC-16 (Modbus)
    Crc16,
    /// CRC-32 (IEEE 802.3)
    Crc32,
    /// XOR 校验
    Xor,
    /// 字节累加和
    Sum,
}

impl ChecksumAlgorithm {
    /// 转换为字符串
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ChecksumAlgorithm::Crc8 => "crc8",
            ChecksumAlgorithm::Crc16 => "crc16",
            ChecksumAlgorithm::Crc32 => "crc32",
            ChecksumAlgorithm::Xor => "xor",
            ChecksumAlgorithm::Sum => "sum",
        }
    }
}

/// 字节序
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    /// 大端序
    Big,
    /// 小端序
    Little,
}

impl Endianness {
    /// 转换为字符串
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Endianness::Big => "big",
            Endianness::Little => "little",
        }
    }
}

/// 结构体属性
#[derive(Debug, Clone, PartialEq)]
pub enum StructAttribute {
    /// 发送方向
    Send,
    /// 接收方向
    Receive,
    /// 字节序
    Endian(Endianness),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::{Type, Value};

    #[test]
    fn test_schema_new() {
        let schema = Schema::new();
        assert!(schema.structs.is_empty());
        assert!(schema.struct_map.is_empty());
    }

    #[test]
    fn test_schema_default() {
        let schema: Schema = Default::default();
        assert!(schema.structs.is_empty());
    }

    #[test]
    fn test_schema_add_struct() {
        let mut schema = Schema::new();
        let def = StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![],
            doc: None,
            endian: None,
        };
        schema.add_struct(def);

        assert_eq!(schema.structs.len(), 1);
        assert!(schema.struct_map.contains_key("Test"));
    }

    #[test]
    fn test_schema_get_struct() {
        let mut schema = Schema::new();
        let def = StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![],
            doc: None,
            endian: None,
        };
        schema.add_struct(def);

        assert!(schema.get_struct("Test").is_some());
        assert!(schema.get_struct("NonExistent").is_none());
    }

    #[test]
    fn test_schema_get_first_receive_struct() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Send".to_string(),
            direction: Some(Direction::Send),
            fields: vec![],
            doc: None,
            endian: None,
        });
        schema.add_struct(StructDef {
            name: "Receive".to_string(),
            direction: Some(Direction::Receive),
            fields: vec![],
            doc: None,
            endian: None,
        });

        let first_recv = schema.get_first_receive_struct();
        assert!(first_recv.is_some());
        assert_eq!(first_recv.unwrap().name, "Receive");
    }

    #[test]
    fn test_schema_get_first_send_struct() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Receive".to_string(),
            direction: Some(Direction::Receive),
            fields: vec![],
            doc: None,
            endian: None,
        });
        schema.add_struct(StructDef {
            name: "Send".to_string(),
            direction: Some(Direction::Send),
            fields: vec![],
            doc: None,
            endian: None,
        });

        let first_send = schema.get_first_send_struct();
        assert!(first_send.is_some());
        assert_eq!(first_send.unwrap().name, "Send");
    }

    #[test]
    fn test_direction_as_str() {
        assert_eq!(Direction::Send.as_str(), "send");
        assert_eq!(Direction::Receive.as_str(), "receive");
    }

    #[test]
    fn test_struct_def_clone() {
        let def = StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Send),
            fields: vec![Field {
                name: "value".to_string(),
                ty: Type::U32,
                value: Some(Value::Integer(42)),
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        };
        let cloned = def.clone();
        assert_eq!(def.name, cloned.name);
        assert_eq!(def.fields.len(), cloned.fields.len());
    }

    #[test]
    fn test_field_clone() {
        let field = Field {
            name: "value".to_string(),
            ty: Type::U32,
            value: Some(Value::Integer(42)),
            attributes: vec![FieldAttribute::Auto(None)],
            doc: None,
        };
        let cloned = field.clone();
        assert_eq!(field.name, cloned.name);
    }

    #[test]
    fn test_field_attribute_variants() {
        let _auto = FieldAttribute::Auto(None);
        let _auto_field = FieldAttribute::Auto(Some("field".to_string()));
        let _len_ref = FieldAttribute::LenRef("field".to_string());
        let _remaining = FieldAttribute::Remaining;
        let _bits = FieldAttribute::Bits(0, 4);
        let _bit = FieldAttribute::Bit(7);
        let _if = FieldAttribute::If("condition".to_string());
        let _checksum = FieldAttribute::Checksum(ChecksumAlgorithm::Crc16);
        let _endian = FieldAttribute::Endian(Endianness::Big);
    }

    #[test]
    fn test_checksum_algorithm_as_str() {
        assert_eq!(ChecksumAlgorithm::Crc8.as_str(), "crc8");
        assert_eq!(ChecksumAlgorithm::Crc16.as_str(), "crc16");
        assert_eq!(ChecksumAlgorithm::Crc32.as_str(), "crc32");
        assert_eq!(ChecksumAlgorithm::Xor.as_str(), "xor");
        assert_eq!(ChecksumAlgorithm::Sum.as_str(), "sum");
    }

    #[test]
    fn test_endianness_as_str() {
        assert_eq!(Endianness::Big.as_str(), "big");
        assert_eq!(Endianness::Little.as_str(), "little");
    }

    #[test]
    fn test_struct_attribute_variants() {
        let _send = StructAttribute::Send;
        let _receive = StructAttribute::Receive;
        let _endian = StructAttribute::Endian(Endianness::Little);
    }

    #[test]
    fn test_direction_clone_and_eq() {
        let dir = Direction::Send;
        let cloned = dir.clone();
        assert_eq!(dir, cloned);
        assert_eq!(dir, Direction::Send);
        assert_ne!(dir, Direction::Receive);
    }
}
