//! 类型和值定义

/// DSL 中支持的类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// 8位无符号整数
    U8,
    /// 16位无符号整数
    U16,
    /// 32位无符号整数
    U32,
    /// 64位无符号整数
    U64,
    /// 128位无符号整数
    U128,

    /// 8位有符号整数
    I8,
    /// 16位有符号整数
    I16,
    /// 32位有符号整数
    I32,
    /// 64位有符号整数
    I64,
    /// 128位有符号整数
    I128,

    /// 32位浮点数
    F32,
    /// 64位浮点数
    F64,

    /// 布尔类型
    Bool,

    /// UTF-8 字符串
    String,

    /// 字节数组
    Bytes,

    /// 变长数组
    Vec(Box<Type>),

    /// 固定长度数组
    Array(Box<Type>, usize),

    /// 自定义结构体类型
    Custom(String),
}

impl Type {
    /// 获取类型的大小（字节）
    ///
    /// 对于变长类型返回 `None`
    #[must_use]
    pub fn size(&self) -> Option<usize> {
        match self {
            Type::U8 | Type::I8 | Type::Bool => Some(1),
            Type::U16 | Type::I16 => Some(2),
            Type::U32 | Type::I32 | Type::F32 => Some(4),
            Type::U64 | Type::I64 | Type::F64 => Some(8),
            Type::U128 | Type::I128 => Some(16),
            Type::Array(inner, count) => inner.size().map(|s| s * count),
            Type::String | Type::Bytes | Type::Vec(_) | Type::Custom(_) => None,
        }
    }

    /// 检查是否为变长类型
    #[must_use]
    pub fn is_variable_length(&self) -> bool {
        matches!(
            self,
            Type::Vec(_) | Type::Bytes | Type::String | Type::Custom(_)
        )
    }
}

/// DSL 中的值
#[derive(Debug, Clone)]
pub enum Value {
    /// 整数
    Integer(i128),
    /// 浮点数
    Float(f64),
    /// 布尔
    Bool(bool),
    /// 字符串
    String(String),
    /// 字节数组
    Bytes(Vec<u8>),
    /// 数组
    Array(Vec<Value>),
    /// 结构体
    Struct(String, Vec<(String, Value)>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_size_primitive() {
        assert_eq!(Type::U8.size(), Some(1));
        assert_eq!(Type::I8.size(), Some(1));
        assert_eq!(Type::Bool.size(), Some(1));
        assert_eq!(Type::U16.size(), Some(2));
        assert_eq!(Type::I16.size(), Some(2));
        assert_eq!(Type::U32.size(), Some(4));
        assert_eq!(Type::I32.size(), Some(4));
        assert_eq!(Type::F32.size(), Some(4));
        assert_eq!(Type::U64.size(), Some(8));
        assert_eq!(Type::I64.size(), Some(8));
        assert_eq!(Type::F64.size(), Some(8));
        assert_eq!(Type::U128.size(), Some(16));
        assert_eq!(Type::I128.size(), Some(16));
    }

    #[test]
    fn test_type_size_array() {
        assert_eq!(Type::Array(Box::new(Type::U8), 4).size(), Some(4));
        assert_eq!(Type::Array(Box::new(Type::U32), 10).size(), Some(40));
    }

    #[test]
    fn test_type_size_variable() {
        assert_eq!(Type::String.size(), None);
        assert_eq!(Type::Bytes.size(), None);
        assert_eq!(Type::Vec(Box::new(Type::U32)).size(), None);
        assert_eq!(Type::Custom("MyType".to_string()).size(), None);
    }

    #[test]
    fn test_type_is_variable_length() {
        assert!(Type::String.is_variable_length());
        assert!(Type::Bytes.is_variable_length());
        assert!(Type::Vec(Box::new(Type::U32)).is_variable_length());
        assert!(Type::Custom("MyType".to_string()).is_variable_length());
        assert!(!Type::U32.is_variable_length());
        assert!(!Type::Array(Box::new(Type::U8), 4).is_variable_length());
    }

    #[test]
    fn test_type_clone() {
        let ty = Type::U32;
        let cloned = ty.clone();
        assert_eq!(ty, cloned);
    }

    #[test]
    fn test_value_clone() {
        let val = Value::Integer(42);
        let cloned = val.clone();
        assert!(matches!(cloned, Value::Integer(42)));
    }

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::U32, Type::U32);
        assert_ne!(Type::U32, Type::U64);
    }
}
