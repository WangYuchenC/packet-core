//! 核心库错误类型定义

use std::fmt;

/// 解析错误类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParseErrorKind {
    /// 词法错误：无效字符、无法识别的标记
    LexicalError,
    /// 语法错误：缺少括号、格式不正确
    SyntaxError,
    /// 语义错误：未定义的类型、重复名称
    SemanticError,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseErrorKind::LexicalError => write!(f, "lexical error"),
            ParseErrorKind::SyntaxError => write!(f, "syntax error"),
            ParseErrorKind::SemanticError => write!(f, "semantic error"),
        }
    }
}

/// 编解码错误类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodecErrorKind {
    /// 编码失败：字段缺少值、类型不匹配
    EncodeFailed,
    /// 解码失败：数据不足、格式错误
    DecodeFailed,
    /// 校验和不匹配
    ChecksumMismatch,
    /// 缓冲区太小
    BufferTooSmall,
}

impl fmt::Display for CodecErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodecErrorKind::EncodeFailed => write!(f, "encode failed"),
            CodecErrorKind::DecodeFailed => write!(f, "decode failed"),
            CodecErrorKind::ChecksumMismatch => write!(f, "checksum mismatch"),
            CodecErrorKind::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

/// 验证失败类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValidationFailureKind {
    /// 发送结构缺少值
    MissingValue,
    /// 重复名称
    DuplicateName,
    /// 未知类型
    UnknownType,
    /// 零大小数组
    ZeroSizeArray,
}

impl fmt::Display for ValidationFailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationFailureKind::MissingValue => write!(f, "missing value"),
            ValidationFailureKind::DuplicateName => write!(f, "duplicate name"),
            ValidationFailureKind::UnknownType => write!(f, "unknown type"),
            ValidationFailureKind::ZeroSizeArray => write!(f, "zero size array"),
        }
    }
}

/// 核心库错误类型
#[derive(Debug, Clone)]
pub enum CoreError {
    /// 解析错误
    Parse {
        /// 错误类型分类
        kind: ParseErrorKind,
        /// 错误位置（如果有）
        location: Option<Location>,
        /// 错误消息
        message: String,
        /// 修复建议
        hint: Option<String>,
    },

    /// 验证错误
    Validation {
        /// 错误代码
        code: String,
        /// 错误类型分类
        kind: Option<ValidationFailureKind>,
        /// 错误消息
        message: String,
    },

    /// 编解码错误
    Codec {
        /// 错误类型分类
        kind: CodecErrorKind,
        /// 错误上下文
        context: String,
        /// 错误消息
        message: String,
    },

    /// IO 错误
    Io {
        /// 操作类型
        operation: String,
        /// 错误消息
        message: String,
    },
}

/// 代码位置信息
#[derive(Debug, Clone, Copy)]
pub struct Location {
    /// 行号
    pub line: usize,
    /// 列号
    pub column: usize,
}

impl CoreError {
    /// 创建解析错误
    pub fn parse(message: impl Into<String>) -> Self {
        CoreError::Parse {
            kind: ParseErrorKind::SyntaxError,
            location: None,
            message: message.into(),
            hint: None,
        }
    }

    /// 创建带类型和位置的解析错误
    pub fn parse_with_kind(
        kind: ParseErrorKind,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) -> Self {
        CoreError::Parse {
            kind,
            location: Some(Location { line, column }),
            message: message.into(),
            hint: None,
        }
    }

    /// 创建带位置的解析错误
    pub fn parse_at(line: usize, column: usize, message: impl Into<String>) -> Self {
        CoreError::Parse {
            kind: ParseErrorKind::SyntaxError,
            location: Some(Location { line, column }),
            message: message.into(),
            hint: None,
        }
    }

    /// 创建带 hint 的解析错误
    pub fn parse_with_hint(
        line: usize,
        column: usize,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        CoreError::Parse {
            kind: ParseErrorKind::SyntaxError,
            location: Some(Location { line, column }),
            message: message.into(),
            hint: Some(hint.into()),
        }
    }

    /// 创建验证错误
    pub fn validation(code: impl Into<String>, message: impl Into<String>) -> Self {
        CoreError::Validation {
            code: code.into(),
            kind: None,
            message: message.into(),
        }
    }

    /// 创建带分类的验证错误
    pub fn validation_with_kind(
        code: impl Into<String>,
        kind: ValidationFailureKind,
        message: impl Into<String>,
    ) -> Self {
        CoreError::Validation {
            code: code.into(),
            kind: Some(kind),
            message: message.into(),
        }
    }

    /// 创建编解码错误
    pub fn codec(context: impl Into<String>, message: impl Into<String>) -> Self {
        CoreError::Codec {
            kind: CodecErrorKind::EncodeFailed,
            context: context.into(),
            message: message.into(),
        }
    }

    /// 创建带类型的编解码错误
    pub fn codec_with_kind(
        kind: CodecErrorKind,
        context: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        CoreError::Codec {
            kind,
            context: context.into(),
            message: message.into(),
        }
    }

    /// 创建 IO 错误
    pub fn io(operation: impl Into<String>, message: impl Into<String>) -> Self {
        CoreError::Io {
            operation: operation.into(),
            message: message.into(),
        }
    }

    /// 获取解析错误类型
    #[must_use]
    pub fn parse_kind(&self) -> Option<ParseErrorKind> {
        match self {
            CoreError::Parse { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// 获取编解码错误类型
    #[must_use]
    pub fn codec_kind(&self) -> Option<CodecErrorKind> {
        match self {
            CoreError::Codec { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// 获取验证失败类型
    #[must_use]
    pub fn validation_kind(&self) -> Option<ValidationFailureKind> {
        match self {
            CoreError::Validation { kind, .. } => *kind,
            _ => None,
        }
    }

    /// 获取错误位置
    #[must_use]
    pub fn location(&self) -> Option<Location> {
        match self {
            CoreError::Parse { location, .. } => *location,
            _ => None,
        }
    }

    /// 获取修复建议
    #[must_use]
    pub fn hint(&self) -> Option<&str> {
        match self {
            CoreError::Parse { hint, .. } => hint.as_deref(),
            _ => None,
        }
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Parse {
                kind,
                location,
                message,
                hint,
            } => {
                if let Some(loc) = location {
                    write!(f, "{} at {}:{}: {}", kind, loc.line, loc.column, message)?;
                } else {
                    write!(f, "{kind}: {message}")?;
                }
                if let Some(h) = hint {
                    write!(f, "\n  = help: {h}")?;
                }
                Ok(())
            }
            CoreError::Validation {
                code,
                kind,
                message,
            } => {
                if let Some(k) = kind {
                    write!(f, "validation error [{code}]: {message} ({k})")
                } else {
                    write!(f, "validation error [{code}]: {message}")
                }
            }
            CoreError::Codec {
                kind,
                context,
                message,
            } => {
                write!(f, "codec error ({kind}) in {context}: {message}")
            }
            CoreError::Io { operation, message } => {
                write!(f, "io error during {operation}: {message}")
            }
        }
    }
}

impl std::error::Error for CoreError {}

/// 结果类型别名
pub type Result<T> = std::result::Result<T, CoreError>;

impl From<std::io::Error> for CoreError {
    fn from(err: std::io::Error) -> Self {
        CoreError::Io {
            operation: "io operation".to_string(),
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error() {
        let err = CoreError::parse("unexpected token");
        assert!(matches!(err, CoreError::Parse { .. }));
        assert_eq!(err.parse_kind(), Some(ParseErrorKind::SyntaxError));
        assert!(format!("{}", err).contains("syntax error"));
        assert!(format!("{}", err).contains("unexpected token"));
    }

    #[test]
    fn test_parse_error_with_location() {
        let err = CoreError::parse_at(10, 5, "unexpected token");
        assert!(matches!(err, CoreError::Parse { .. }));
        assert!(format!("{}", err).contains("10:5"));
    }

    #[test]
    fn test_parse_error_with_kind() {
        let err = CoreError::parse_with_kind(
            ParseErrorKind::LexicalError,
            3,
            10,
            "invalid character '@'",
        );
        assert_eq!(err.parse_kind(), Some(ParseErrorKind::LexicalError));
        assert!(format!("{}", err).contains("lexical error"));
    }

    #[test]
    fn test_parse_error_with_hint() {
        let err =
            CoreError::parse_with_hint(5, 12, "旧语法已移除", "请使用函数风格: #[len_ref(count)]");
        assert!(matches!(err, CoreError::Parse { .. }));
        assert_eq!(err.hint(), Some("请使用函数风格: #[len_ref(count)]"));
        let display = format!("{}", err);
        assert!(display.contains("旧语法已移除"));
        assert!(display.contains("help:"));
    }

    #[test]
    fn test_validation_error() {
        let err = CoreError::validation("E001", "field missing");
        assert!(matches!(err, CoreError::Validation { .. }));
        assert!(format!("{}", err).contains("E001"));
    }

    #[test]
    fn test_validation_error_with_kind() {
        let err = CoreError::validation_with_kind(
            "E002",
            ValidationFailureKind::MissingValue,
            "send struct field has no value",
        );
        assert_eq!(
            err.validation_kind(),
            Some(ValidationFailureKind::MissingValue)
        );
        assert!(format!("{}", err).contains("missing value"));
    }

    #[test]
    fn test_codec_error() {
        let err = CoreError::codec("encode", "buffer overflow");
        assert!(matches!(err, CoreError::Codec { .. }));
        assert_eq!(err.codec_kind(), Some(CodecErrorKind::EncodeFailed));
        assert!(format!("{}", err).contains("encode"));
    }

    #[test]
    fn test_codec_error_with_kind() {
        let err = CoreError::codec_with_kind(
            CodecErrorKind::ChecksumMismatch,
            "decode",
            "CRC32 mismatch",
        );
        assert_eq!(err.codec_kind(), Some(CodecErrorKind::ChecksumMismatch));
        assert!(format!("{}", err).contains("checksum mismatch"));
    }

    #[test]
    fn test_io_error() {
        let err = CoreError::io("read file", "permission denied");
        assert!(matches!(err, CoreError::Io { .. }));
        assert!(format!("{}", err).contains("read file"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let core_err: CoreError = io_err.into();
        assert!(matches!(core_err, CoreError::Io { .. }));
        assert!(format!("{}", core_err).contains("file not found"));
    }

    #[test]
    fn test_location_struct() {
        let loc = Location {
            line: 5,
            column: 10,
        };
        assert_eq!(loc.line, 5);
        assert_eq!(loc.column, 10);
    }

    #[test]
    fn test_core_error_implements_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(CoreError::parse("test"));
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_core_error_implements_clone() {
        let err = CoreError::validation("E001", "test error");
        let cloned = err.clone();
        assert_eq!(format!("{}", err), format!("{}", cloned));
    }

    #[test]
    fn test_error_kind_display() {
        assert_eq!(format!("{}", ParseErrorKind::LexicalError), "lexical error");
        assert_eq!(format!("{}", CodecErrorKind::DecodeFailed), "decode failed");
        assert_eq!(
            format!("{}", ValidationFailureKind::ZeroSizeArray),
            "zero size array"
        );
    }
}
