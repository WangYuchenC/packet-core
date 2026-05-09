//! 词法分析器
//!
//! 使用 logos crate 实现的词法分析器。

use logos::Logos;
use std::hash::{Hash, Hasher};

/// Token 类型
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\r\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
pub enum Token {
    /// 文档注释 (/// ...)
    #[regex(r"///([^\n]*)", |lex| lex.slice()[3..].trim().to_string())]
    DocComment(String),
    /// struct 关键字
    #[token("struct")]
    Struct,
    /// pub 关键字
    #[token("pub")]
    Pub,
    /// import 关键字
    #[token("import")]
    Import,
    /// true 关键字
    #[token("true")]
    True,
    /// false 关键字
    #[token("false")]
    False,

    /// u8 类型
    #[token("u8")]
    U8,
    /// u16 类型
    #[token("u16")]
    U16,
    /// u32 类型
    #[token("u32")]
    U32,
    /// u64 类型
    #[token("u64")]
    U64,
    /// u128 类型
    #[token("u128")]
    U128,
    /// i8 类型
    #[token("i8")]
    I8,
    /// i16 类型
    #[token("i16")]
    I16,
    /// i32 类型
    #[token("i32")]
    I32,
    /// i64 类型
    #[token("i64")]
    I64,
    /// i128 类型
    #[token("i128")]
    I128,
    /// f32 类型
    #[token("f32")]
    F32,
    /// f64 类型
    #[token("f64")]
    F64,
    /// bool 类型
    #[token("bool")]
    Bool,
    /// String 类型
    #[token("String")]
    String,
    /// Bytes 类型
    #[token("Bytes")]
    Bytes,
    /// Vec 类型
    #[token("Vec")]
    Vec,

    /// 左花括号
    #[token("{")]
    BraceOpen,
    /// 右花括号
    #[token("}")]
    BraceClose,
    /// 左方括号
    #[token("[")]
    BracketOpen,
    /// 右方括号
    #[token("]")]
    BracketClose,
    /// 左圆括号
    #[token("(")]
    ParenOpen,
    /// 右圆括号
    #[token(")")]
    ParenClose,
    /// 冒号
    #[token(":")]
    Colon,
    /// 分号
    #[token(";")]
    Semicolon,
    /// 逗号
    #[token(",")]
    Comma,
    /// 等号
    #[token("=")]
    Equals,
    /// 双冒号
    #[token("::")]
    DoubleColon,
    /// 点号
    #[token(".")]
    Dot,
    /// 井号
    #[token("#")]
    Hash,
    /// 感叹号
    #[token("!")]
    Exclamation,
    /// 小于号
    #[token("<")]
    LessThan,
    /// 大于号
    #[token(">")]
    GreaterThan,

    /// 标识符
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    /// 整数
    #[regex(r"-?\d+", |lex| lex.slice().parse::<i128>().ok())]
    Integer(Option<i128>),

    /// 浮点数
    #[regex(r"-?\d+\.\d+", |lex| lex.slice().parse::<f64>().ok())]
    Float(Option<f64>),

    /// 字符串字面量
    #[regex(r#""[^"]*""#, |lex| lex.slice()[1..lex.slice().len()-1].to_string())]
    StringLiteral(String),

    /// 十六进制数字
    #[regex(r"0x[0-9a-fA-F]+", |lex| u64::from_str_radix(&lex.slice()[2..], 16).ok())]
    HexNumber(Option<u64>),
}

impl Eq for Token {}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // 使用 discriminant 来哈希变体类型
        std::mem::discriminant(self).hash(state);

        // 对于有数据的变体，哈希数据（跳过 Float）
        match self {
            Token::Ident(s) => s.hash(state),
            Token::Integer(n) => n.hash(state),
            Token::Float(_) => 0.hash(state), // Float 使用占位符哈希
            Token::StringLiteral(s) => s.hash(state),
            Token::HexNumber(n) => n.hash(state),
            _ => {} // 其他变体只有 discriminant
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    #[test]
    fn test_keywords() {
        let mut lex = Token::lexer("struct pub true false");
        assert!(matches!(lex.next(), Some(Ok(Token::Struct))));
        assert!(matches!(lex.next(), Some(Ok(Token::Pub))));
        assert!(matches!(lex.next(), Some(Ok(Token::True))));
        assert!(matches!(lex.next(), Some(Ok(Token::False))));
    }

    #[test]
    fn test_types() {
        let mut lex =
            Token::lexer("u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 bool String Bytes Vec");
        assert!(matches!(lex.next(), Some(Ok(Token::U8))));
        assert!(matches!(lex.next(), Some(Ok(Token::U16))));
        assert!(matches!(lex.next(), Some(Ok(Token::U32))));
        assert!(matches!(lex.next(), Some(Ok(Token::U64))));
        assert!(matches!(lex.next(), Some(Ok(Token::U128))));
        assert!(matches!(lex.next(), Some(Ok(Token::I8))));
        assert!(matches!(lex.next(), Some(Ok(Token::I16))));
        assert!(matches!(lex.next(), Some(Ok(Token::I32))));
        assert!(matches!(lex.next(), Some(Ok(Token::I64))));
        assert!(matches!(lex.next(), Some(Ok(Token::I128))));
        assert!(matches!(lex.next(), Some(Ok(Token::F32))));
        assert!(matches!(lex.next(), Some(Ok(Token::F64))));
        assert!(matches!(lex.next(), Some(Ok(Token::Bool))));
        assert!(matches!(lex.next(), Some(Ok(Token::String))));
        assert!(matches!(lex.next(), Some(Ok(Token::Bytes))));
        assert!(matches!(lex.next(), Some(Ok(Token::Vec))));
    }

    #[test]
    fn test_punctuation() {
        let mut lex = Token::lexer("{ } [ ] ( ) : ; , = :: . #");
        assert!(matches!(lex.next(), Some(Ok(Token::BraceOpen))));
        assert!(matches!(lex.next(), Some(Ok(Token::BraceClose))));
        assert!(matches!(lex.next(), Some(Ok(Token::BracketOpen))));
        assert!(matches!(lex.next(), Some(Ok(Token::BracketClose))));
        assert!(matches!(lex.next(), Some(Ok(Token::ParenOpen))));
        assert!(matches!(lex.next(), Some(Ok(Token::ParenClose))));
        assert!(matches!(lex.next(), Some(Ok(Token::Colon))));
        assert!(matches!(lex.next(), Some(Ok(Token::Semicolon))));
        assert!(matches!(lex.next(), Some(Ok(Token::Comma))));
        assert!(matches!(lex.next(), Some(Ok(Token::Equals))));
        assert!(matches!(lex.next(), Some(Ok(Token::DoubleColon))));
        assert!(matches!(lex.next(), Some(Ok(Token::Dot))));
        assert!(matches!(lex.next(), Some(Ok(Token::Hash))));
    }

    #[test]
    fn test_identifiers() {
        let mut lex = Token::lexer("foo _bar Baz123");
        assert!(matches!(lex.next(), Some(Ok(Token::Ident(s))) if s == "foo"));
        assert!(matches!(lex.next(), Some(Ok(Token::Ident(s))) if s == "_bar"));
        assert!(matches!(lex.next(), Some(Ok(Token::Ident(s))) if s == "Baz123"));
    }

    #[test]
    fn test_numbers() {
        let mut lex = Token::lexer("42 -10 3.14 -2.5 0xFF");
        assert!(matches!(lex.next(), Some(Ok(Token::Integer(Some(42))))));
        assert!(matches!(lex.next(), Some(Ok(Token::Integer(Some(-10))))));
        assert!(matches!(lex.next(), Some(Ok(Token::Float(Some(3.14))))));
        assert!(matches!(lex.next(), Some(Ok(Token::Float(Some(-2.5))))));
        assert!(matches!(lex.next(), Some(Ok(Token::HexNumber(Some(255))))));
    }

    #[test]
    fn test_string() {
        let mut lex = Token::lexer("\"hello world\"");
        assert!(matches!(lex.next(), Some(Ok(Token::StringLiteral(s))) if s == "hello world"));
    }

    #[test]
    fn test_skip_whitespace() {
        let mut lex = Token::lexer("  struct   pub  ");
        assert!(matches!(lex.next(), Some(Ok(Token::Struct))));
        assert!(matches!(lex.next(), Some(Ok(Token::Pub))));
        assert!(lex.next().is_none());
    }

    #[test]
    fn test_skip_comment() {
        let mut lex = Token::lexer("struct // this is a comment\npub");
        assert!(matches!(lex.next(), Some(Ok(Token::Struct))));
        assert!(matches!(lex.next(), Some(Ok(Token::Pub))));
    }
}
