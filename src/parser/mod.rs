//! Parser 模块
//!
//! 提供 `.pkt` DSL 的解析功能。

pub mod lexer;
pub mod parser_impl;

use crate::ast::Schema;
use crate::error::{CoreError, Result};
use lexer::Token;
use logos::Logos;

/// 将字节偏移量转换为行号和列号
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.lines().count().max(1);
    let column = prefix.rfind('\n').map_or(offset + 1, |i| offset - i);
    (line, column)
}

/// 解析 Schema 字符串
///
/// # Arguments
///
/// * `source` - DSL 源代码
///
/// # Returns
///
/// 成功返回 `Schema`，失败返回 `CoreError`
///
/// # Examples
///
/// ```rust
/// use packet_core::parse_schema;
///
/// let source = r#"
/// #[send]
/// struct Test {
///     value: u32 = 42,
/// }
/// "#;
///
/// let schema = parse_schema(source).unwrap();
/// assert!(schema.get_struct("Test").is_some());
/// ```
pub fn parse_schema(source: &str) -> Result<Schema> {
    // 1. 词法分析
    let lexer = Token::lexer(source);
    let mut tokens = Vec::new();
    let mut spans: Vec<std::ops::Range<usize>> = Vec::new();

    for result in lexer.spanned() {
        match result {
            (Ok(token), span) => {
                tokens.push(token);
                spans.push(span);
            }
            (Err(()), span) => {
                let (line, column) = offset_to_line_col(source, span.start);
                return Err(CoreError::parse_at(line, column, "invalid token"));
            }
        }
    }

    // 2. 语法分析
    match parser_impl::parse_schema_tokens(&tokens, &spans) {
        Ok(schema) => Ok(schema),
        Err(errors) => {
            // 取第一个错误（最相关）
            let e = &errors[0];
            let span = e.span();
            let (line, column) = offset_to_line_col(source, span.start);

            // 提取 chumsky 错误中的关键信息
            let message = match e.reason() {
                chumsky::error::SimpleReason::Custom(msg) => msg.clone(),
                chumsky::error::SimpleReason::Unexpected => {
                    let found = e
                        .found()
                        .map_or_else(|| "end of file".to_string(), |t| format!("`{t:?}`"));
                    let expected: Vec<String> = e
                        .expected()
                        .filter_map(|exp| match exp {
                            Some(Token::Ident(s)) => Some(format!("`{s}`")),
                            Some(t) => Some(format!("`{t:?}`")),
                            None => None,
                        })
                        .collect();

                    if expected.is_empty() {
                        format!("unexpected {found}")
                    } else {
                        format!("expected {}, but found {}", expected.join(" or "), found)
                    }
                }
                chumsky::error::SimpleReason::Unclosed { span: _, delimiter } => {
                    format!("unclosed delimiter `{delimiter:?}`")
                }
            };

            Err(CoreError::parse_at(line, column, message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_schema("");
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert!(schema.structs.is_empty());
    }

    #[test]
    fn test_parse_simple_struct() {
        let source = r#"
            struct Test {
                value: u32 = 42,
            }
        "#;
        let result = parse_schema(source);
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert!(schema.get_struct("Test").is_some());
    }

    #[test]
    fn test_parse_multiple_fields() {
        let source = r#"
            struct Data {
                id: u32 = 1,
                name: String = "test",
                active: bool = true,
            }
        "#;
        let result = parse_schema(source);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Data").unwrap();
        assert_eq!(def.fields.len(), 3);
    }

    #[test]
    fn test_parse_with_comments() {
        let source = r#"
            // This is a comment
            struct Test {
                value: u32, // inline comment
            }
        "#;
        let result = parse_schema(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_complex_types() {
        let source = r#"
            struct Complex {
                arr: [u8; 4],
                vec: Vec<u32>,
            }
        "#;
        let result = parse_schema(source);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Complex").unwrap();
        assert_eq!(def.fields.len(), 2);
    }

    #[test]
    fn test_parse_invalid_token() {
        let source = "struct @invalid";
        let result = parse_schema(source);
        assert!(result.is_err());
    }
}
