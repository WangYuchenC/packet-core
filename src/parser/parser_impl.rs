//! 语法分析器实现
//!
//! 使用 chumsky crate 实现的语法分析器。

// chumsky 的 select! 宏会产生较大的错误类型，这是上游设计，此处允许
#![allow(clippy::result_large_err)]

use super::lexer::Token;
use crate::ast::{
    ChecksumAlgorithm, Direction, Endianness, Field, FieldAttribute, FileAttribute, Import, Schema,
    StructDef, Type, Value,
};
use chumsky::prelude::*;

/// 解析 Schema
pub fn parse_schema_tokens(
    tokens: &[Token],
    spans: &[std::ops::Range<usize>],
) -> Result<Schema, Vec<Simple<Token>>> {
    let eof_span = spans.last().map(|s| s.end..s.end + 1).unwrap_or(0..1);
    schema_parser().parse(chumsky::Stream::from_iter(
        eof_span,
        tokens.iter().cloned().zip(spans.iter().cloned()),
    ))
}

/// Schema 解析器
fn schema_parser() -> impl Parser<Token, Schema, Error = Simple<Token>> + Clone {
    // 解析文件级属性、导入语句和结构体定义
    file_attribute_parser()
        .repeated()
        .then(import_parser().repeated())
        .then(struct_def_parser().repeated())
        .then_ignore(end())
        .map(
            |((file_attrs, imports), structs): (
                (Vec<FileAttribute>, Vec<Import>),
                Vec<StructDef>,
            )| {
                let mut schema = Schema::new();
                for attr in file_attrs {
                    schema.add_file_attribute(attr);
                }
                for import in imports {
                    schema.add_import(import);
                }
                for def in structs {
                    schema.add_struct(def);
                }
                schema
            },
        )
}

/// 文档注释解析器
fn doc_comment_parser() -> impl Parser<Token, Vec<String>, Error = Simple<Token>> + Clone {
    select! { Token::DocComment(text) => text }.repeated()
}

/// Import 解析器 - 支持规范语法 import("path.pkt")
fn import_parser() -> impl Parser<Token, Import, Error = Simple<Token>> + Clone {
    doc_comment_parser()
        .then_ignore(just(Token::Import))
        .then(
            // 规范语法: import("path")
            just(Token::ParenOpen)
                .ignore_then(select! { Token::StringLiteral(path) => path })
                .then_ignore(just(Token::ParenClose)),
        )
        .map(|(doc_comments, path)| {
            let doc = if doc_comments.is_empty() {
                None
            } else {
                Some(doc_comments.join("\n"))
            };
            Import { path, doc }
        })
}

/// 文件级属性解析器 - 支持规范函数风格语法
fn file_attribute_parser() -> impl Parser<Token, FileAttribute, Error = Simple<Token>> + Clone {
    just(Token::Hash)
        .ignore_then(just(Token::Exclamation))
        .ignore_then(just(Token::BracketOpen))
        .ignore_then(choice((
            // #![version("x.y.z")]
            version_file_attr_parser(),
            // #![endian(big)] / #![endian(little)]
            endian_file_attr_parser(),
            // #![import_path("lib/")]
            import_path_file_attr_parser(),
            // #![doc("...")]
            doc_file_attr_parser(),
            // #![prefix(disable)]
            prefix_file_attr_parser(),
        )))
        .then_ignore(just(Token::BracketClose))
}

/// #![version("x.y.z")] 解析器
fn version_file_attr_parser() -> impl Parser<Token, FileAttribute, Error = Simple<Token>> + Clone {
    just(Token::Ident("version".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .then(select! { Token::StringLiteral(version) => version })
        .then_ignore(just(Token::ParenClose))
        .map(|(_, version)| FileAttribute::Version(version))
}

/// #![endian(big)] / #![endian(little)] 解析器
fn endian_file_attr_parser() -> impl Parser<Token, FileAttribute, Error = Simple<Token>> + Clone {
    just(Token::Ident("endian".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! {
            Token::Ident(name) if name == "big" => Endianness::Big,
            Token::Ident(name) if name == "little" => Endianness::Little,
        })
        .then_ignore(just(Token::ParenClose))
        .map(FileAttribute::Endian)
}

/// #![`import_path("prefix`/")] 解析器
fn import_path_file_attr_parser() -> impl Parser<Token, FileAttribute, Error = Simple<Token>> + Clone
{
    just(Token::Ident("import_path".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .then(select! { Token::StringLiteral(path) => path })
        .then_ignore(just(Token::ParenClose))
        .map(|(_, path)| FileAttribute::ImportPath(path))
}

/// #![doc("...")] 解析器
fn doc_file_attr_parser() -> impl Parser<Token, FileAttribute, Error = Simple<Token>> + Clone {
    just(Token::Ident("doc".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .then(select! { Token::StringLiteral(doc) => doc })
        .then_ignore(just(Token::ParenClose))
        .map(|(_, doc)| FileAttribute::Doc(doc))
}

/// #![prefix(disable)] 解析器
fn prefix_file_attr_parser() -> impl Parser<Token, FileAttribute, Error = Simple<Token>> + Clone {
    just(Token::Ident("prefix".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! {
            Token::Ident(name) if name == "disable" => FileAttribute::PrefixDisabled,
        })
        .then_ignore(just(Token::ParenClose))
}

/// 结构体定义解析器 - 移除 pub 支持
fn struct_def_parser() -> impl Parser<Token, StructDef, Error = Simple<Token>> + Clone {
    doc_comment_parser()
        .then(struct_attr_parser().repeated())
        .then_ignore(just(Token::Struct))
        .then(select! { Token::Ident(name) => name })
        .then(
            just(Token::BraceOpen)
                .ignore_then(field_parser().repeated())
                .then_ignore(just(Token::BraceClose)),
        )
        .try_map(|(((doc_comments, attrs), name), fields), _span| {
            let has_send = attrs.contains(&StructAttr::Send);
            let has_receive = attrs.contains(&StructAttr::Receive);
            if has_send && has_receive {
                return Err(Simple::custom(
                    _span,
                    "struct cannot have both #[send] and #[receive] attributes",
                ));
            }

            let direction = if has_send {
                Some(Direction::Send)
            } else if has_receive {
                Some(Direction::Receive)
            } else {
                None
            };
            let endian = attrs.iter().find_map(|attr| match attr {
                StructAttr::Endian(e) => Some(*e),
                _ => None,
            });
            let doc = if doc_comments.is_empty() {
                None
            } else {
                Some(doc_comments.join("\n"))
            };
            Ok(StructDef {
                name,
                direction,
                fields,
                doc,
                endian,
            })
        })
}

/// 结构体属性类型 - 移除 Pub
#[derive(Debug, Clone, PartialEq)]
enum StructAttr {
    Send,
    Receive,
    Endian(Endianness),
}

/// 字段属性类型 - Auto 改为 Option<String>，移除 Version
#[derive(Debug, Clone, PartialEq)]
enum FieldAttr {
    Auto(Option<String>),
    LenRef(String),
    Remaining,
    Bits(usize, usize),
    Bit(usize),
    If(String),
    Checksum(ChecksumAlgorithm),
    Endian(Endianness),
}

/// 结构体属性解析器 - 移除 pub 支持
fn struct_attr_parser() -> impl Parser<Token, StructAttr, Error = Simple<Token>> + Clone {
    just(Token::Hash)
        .ignore_then(just(Token::BracketOpen))
        .ignore_then(
            select! {
                Token::Ident(name) if name == "send" => StructAttr::Send,
                Token::Ident(name) if name == "receive" => StructAttr::Receive,
            }
            .or(struct_endian_attr_parser()),
        )
        .then_ignore(just(Token::BracketClose))
}

/// 结构体 endian 属性解析器
fn struct_endian_attr_parser() -> impl Parser<Token, StructAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("endian".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! {
            Token::Ident(name) if name == "big" => StructAttr::Endian(Endianness::Big),
            Token::Ident(name) if name == "little" => StructAttr::Endian(Endianness::Little),
        })
        .then_ignore(just(Token::ParenClose))
}

/// 字段属性解析器 - 支持规范函数风格语法
fn field_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Hash)
        .ignore_then(just(Token::BracketOpen))
        .ignore_then(choice((
            // #[auto] / #[auto(field)]
            auto_attr_parser(),
            // #[remaining]
            just(Token::Ident("remaining".to_string())).to(FieldAttr::Remaining),
            // #[len_ref(field)]
            len_ref_attr_parser(),
            // #[if(condition)]
            if_attr_parser(),
            // #[bits(start, end)]
            bits_attr_parser(),
            // #[bit(pos)]
            bit_attr_parser(),
            // #[endian(big)] / #[endian(little)]
            field_endian_attr_parser(),
            // #[checksum(algo)]
            checksum_attr_parser(),
        )))
        .then_ignore(just(Token::BracketClose))
}

/// #[auto] / #[auto(field)] 解析器
fn auto_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("auto".to_string())).ignore_then(
        just(Token::ParenOpen)
            .ignore_then(select! { Token::Ident(name) => name })
            .then_ignore(just(Token::ParenClose))
            .map(|name| FieldAttr::Auto(Some(name)))
            .or(empty().to(FieldAttr::Auto(None))),
    )
}

/// #[`len_ref(field)`] 解析器
fn len_ref_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("len_ref".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! { Token::Ident(name) => name })
        .then_ignore(just(Token::ParenClose))
        .map(FieldAttr::LenRef)
}

/// #[if(condition)] 解析器
fn if_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("if".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! { Token::Ident(name) => name })
        .then_ignore(just(Token::ParenClose))
        .map(FieldAttr::If)
}

/// #[bits(start, end)] 解析器
fn bits_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("bits".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! { Token::Integer(Some(n)) => n as usize })
        .then_ignore(just(Token::Comma))
        .then(select! { Token::Integer(Some(n)) => n as usize })
        .then_ignore(just(Token::ParenClose))
        .map(|(start, end)| FieldAttr::Bits(start, end))
}

/// #[bit(pos)] 解析器
fn bit_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("bit".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! { Token::Integer(Some(n)) => n as usize })
        .then_ignore(just(Token::ParenClose))
        .map(FieldAttr::Bit)
}

/// 字段 endian 属性解析器
fn field_endian_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("endian".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! {
            Token::Ident(name) if name == "big" => FieldAttr::Endian(Endianness::Big),
            Token::Ident(name) if name == "little" => FieldAttr::Endian(Endianness::Little),
        })
        .then_ignore(just(Token::ParenClose))
}

/// #[checksum(algo)] 解析器
fn checksum_attr_parser() -> impl Parser<Token, FieldAttr, Error = Simple<Token>> + Clone {
    just(Token::Ident("checksum".to_string()))
        .ignore_then(just(Token::ParenOpen))
        .ignore_then(select! {
            Token::Ident(name) => name,
        })
        .then_ignore(just(Token::ParenClose))
        .map(|name| match name.as_str() {
            "crc8" => FieldAttr::Checksum(ChecksumAlgorithm::Crc8),
            "crc16" => FieldAttr::Checksum(ChecksumAlgorithm::Crc16),
            "crc32" => FieldAttr::Checksum(ChecksumAlgorithm::Crc32),
            "xor" => FieldAttr::Checksum(ChecksumAlgorithm::Xor),
            "sum" => FieldAttr::Checksum(ChecksumAlgorithm::Sum),
            _ => panic!("Unknown checksum algorithm: {name}"),
        })
}

/// 字段解析器
fn field_parser() -> impl Parser<Token, Field, Error = Simple<Token>> + Clone {
    doc_comment_parser()
        .then(field_attr_parser().repeated())
        .then(select! { Token::Ident(name) => name })
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .then(just(Token::Equals).ignore_then(value_parser()).or_not())
        .then_ignore(just(Token::Comma).or_not())
        .map(|((((doc_comments, field_attrs), name), ty), value)| {
            // 转换 FieldAttr 到 FieldAttribute
            let attributes: Vec<FieldAttribute> = field_attrs
                .into_iter()
                .map(|attr| match attr {
                    FieldAttr::Auto(opt) => FieldAttribute::Auto(opt),
                    FieldAttr::LenRef(s) => FieldAttribute::LenRef(s),
                    FieldAttr::Remaining => FieldAttribute::Remaining,
                    FieldAttr::Bits(start, end) => FieldAttribute::Bits(start, end),
                    FieldAttr::Bit(pos) => FieldAttribute::Bit(pos),
                    FieldAttr::If(s) => FieldAttribute::If(s),
                    FieldAttr::Checksum(algo) => FieldAttribute::Checksum(algo),
                    FieldAttr::Endian(endian) => FieldAttribute::Endian(endian),
                })
                .collect();
            let doc = if doc_comments.is_empty() {
                None
            } else {
                Some(doc_comments.join("\n"))
            };
            Field {
                name,
                ty,
                value,
                attributes,
                doc,
            }
        })
}

/// 类型解析器
fn type_parser() -> impl Parser<Token, Type, Error = Simple<Token>> + Clone {
    recursive(|ty| {
        let primitive = choice((
            just(Token::U8).to(Type::U8),
            just(Token::U16).to(Type::U16),
            just(Token::U32).to(Type::U32),
            just(Token::U64).to(Type::U64),
            just(Token::U128).to(Type::U128),
            just(Token::I8).to(Type::I8),
            just(Token::I16).to(Type::I16),
            just(Token::I32).to(Type::I32),
            just(Token::I64).to(Type::I64),
            just(Token::I128).to(Type::I128),
            just(Token::F32).to(Type::F32),
            just(Token::F64).to(Type::F64),
            just(Token::Bool).to(Type::Bool),
            just(Token::String).to(Type::String),
            just(Token::Bytes).to(Type::Bytes),
        ));

        let custom = select! { Token::Ident(name) => Type::Custom(name) };

        let vec = just(Token::Vec)
            .ignore_then(just(Token::LessThan))
            .ignore_then(ty.clone())
            .then_ignore(just(Token::GreaterThan))
            .map(|inner| Type::Vec(Box::new(inner)));

        let array = just(Token::BracketOpen)
            .ignore_then(ty.clone())
            .then_ignore(just(Token::Semicolon))
            .then(select! { Token::Integer(Some(n)) => n as usize })
            .then_ignore(just(Token::BracketClose))
            .map(|(inner, count)| Type::Array(Box::new(inner), count));

        choice((primitive, custom, vec, array))
    })
}

/// 值解析器
fn value_parser() -> impl Parser<Token, Value, Error = Simple<Token>> + Clone {
    recursive(|val| {
        let integer = select! {
            Token::Integer(Some(n)) => Value::Integer(n),
        };

        let float = select! {
            Token::Float(Some(f)) => Value::Float(f),
        };

        let boolean = choice((
            just(Token::True).to(Value::Bool(true)),
            just(Token::False).to(Value::Bool(false)),
        ));

        let string = select! {
            Token::StringLiteral(s) => Value::String(s),
        };

        let hex_bytes = select! {
            Token::HexNumber(Some(n)) => Value::Integer(i128::from(n)),
        };

        let array = val
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .delimited_by(just(Token::BracketOpen), just(Token::BracketClose))
            .map(Value::Array);

        // 结构体初始化: TypeName { field: value, ... }
        let struct_init = select! { Token::Ident(name) => name }
            .then(
                select! { Token::Ident(field) => field }
                    .then_ignore(just(Token::Colon))
                    .then(val.clone())
                    .separated_by(just(Token::Comma))
                    .allow_trailing()
                    .delimited_by(just(Token::BraceOpen), just(Token::BraceClose)),
            )
            .map(|(name, fields)| Value::Struct(name, fields));

        choice((
            integer,
            float,
            boolean,
            string,
            hex_bytes,
            array,
            struct_init,
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    fn lex(input: &str) -> (Vec<Token>, Vec<std::ops::Range<usize>>) {
        let mut tokens = Vec::new();
        let mut spans = Vec::new();
        for (result, span) in Token::lexer(input).spanned() {
            if let Ok(token) = result {
                tokens.push(token);
                spans.push(span);
            }
        }
        (tokens, spans)
    }

    #[test]
    fn test_parse_empty_schema() {
        let (tokens, spans) = lex("");
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert!(schema.structs.is_empty());
    }

    #[test]
    fn test_parse_simple_struct() {
        let input = r#"
            struct Test {
                value: u32,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.structs.len(), 1);
        assert!(schema.get_struct("Test").is_some());
    }

    #[test]
    fn test_parse_struct_with_value() {
        let input = r#"
            struct Test {
                value: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Test").unwrap();
        assert_eq!(def.fields.len(), 1);
        assert!(def.fields[0].value.is_some());
    }

    #[test]
    fn test_parse_multiple_structs() {
        let input = r#"
            struct Foo {
                a: u8,
            }
            struct Bar {
                b: u16,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        assert_eq!(schema.structs.len(), 2);
    }

    #[test]
    fn test_type_parser() {
        let input = "u8";
        let (tokens, _spans) = lex(input);
        let result = type_parser().parse(chumsky::Stream::from_iter(
            tokens.len()..tokens.len(),
            tokens.iter().cloned().map(|t| (t, 0..tokens.len())),
        ));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::U8);
    }

    #[test]
    fn test_value_parser_integer() {
        let input = "42";
        let (tokens, _spans) = lex(input);
        let result = value_parser().parse(chumsky::Stream::from_iter(
            tokens.len()..tokens.len(),
            tokens.iter().cloned().map(|t| (t, 0..tokens.len())),
        ));
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Integer(42)));
    }

    #[test]
    fn test_value_parser_bool() {
        let input = "true";
        let (tokens, _spans) = lex(input);
        let result = value_parser().parse(chumsky::Stream::from_iter(
            tokens.len()..tokens.len(),
            tokens.iter().cloned().map(|t| (t, 0..tokens.len())),
        ));
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Value::Bool(true)));
    }

    #[test]
    fn test_parse_struct_with_send_attribute() {
        let input = r#"
            #[send]
            struct Test {
                value: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Test").unwrap();
        assert_eq!(def.direction, Some(Direction::Send));
    }

    #[test]
    fn test_parse_struct_with_receive_attribute() {
        let input = r#"
            #[receive]
            struct Test {
                value: u32,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Test").unwrap();
        assert_eq!(def.direction, Some(Direction::Receive));
    }

    #[test]
    fn test_parse_complex_nested_types() {
        let input = r#"
            struct Test {
                nested_vec: Vec<Vec<u32>>,
                nested_array: [[u8; 4]; 2],
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Test").unwrap();
        assert_eq!(def.fields.len(), 2);
    }

    // ========== 字段属性测试 ==========

    #[test]
    fn test_parse_auto_attribute() {
        let input = r#"
            #[send]
            struct Packet {
                data: Vec<u8> = [1, 2, 3],
                #[auto]
                length: u16,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Packet").unwrap();
        assert_eq!(def.fields.len(), 2);
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::Auto(None)));
    }

    #[test]
    fn test_parse_auto_field_attribute() {
        let input = r#"
            #[send]
            struct VariableData {
                #[auto(items)]
                count: u8,
                items: Vec<u32> = [100, 200, 300],
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("VariableData").unwrap();
        assert!(def.fields[0]
            .attributes
            .contains(&FieldAttribute::Auto(Some("items".to_string()))));
    }

    #[test]
    fn test_parse_len_ref_attribute() {
        let input = r#"
            #[receive]
            struct VariableData {
                count: u8,
                #[len_ref(count)]
                items: Vec<u32>,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("VariableData").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::LenRef("count".to_string())));
    }

    #[test]
    fn test_parse_remaining_attribute() {
        let input = r#"
            #[receive]
            struct Message {
                header: u32,
                #[remaining]
                payload: Bytes,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Message").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::Remaining));
    }

    #[test]
    fn test_parse_endian_attribute() {
        let input = r#"
            #[send]
            struct Packet {
                #[endian(little)]
                value: u32 = 0x12345678,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("Packet").unwrap();
        assert!(def.fields[0]
            .attributes
            .contains(&FieldAttribute::Endian(Endianness::Little)));
    }

    #[test]
    fn test_parse_checksum_crc8_attribute() {
        let input = r#"
            #[send]
            struct ChecksumPacket {
                data: u16 = 0x1234,
                #[checksum(crc8)]
                crc: u8,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("ChecksumPacket").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::Checksum(ChecksumAlgorithm::Crc8)));
    }

    #[test]
    fn test_parse_checksum_crc16_attribute() {
        let input = r#"
            #[send]
            struct ChecksumPacket {
                data: u16 = 0x1234,
                #[checksum(crc16)]
                crc: u16,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("ChecksumPacket").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::Checksum(ChecksumAlgorithm::Crc16)));
    }

    #[test]
    fn test_parse_checksum_crc32_attribute() {
        let input = r#"
            #[send]
            struct ChecksumPacket {
                data: u32 = 0x12345678,
                #[checksum(crc32)]
                crc: u32,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("ChecksumPacket").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::Checksum(ChecksumAlgorithm::Crc32)));
    }

    #[test]
    fn test_parse_checksum_xor_attribute() {
        let input = r#"
            #[send]
            struct ChecksumPacket {
                data: u16 = 0x1234,
                #[checksum(xor)]
                checksum: u8,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("ChecksumPacket").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::Checksum(ChecksumAlgorithm::Xor)));
    }

    #[test]
    fn test_parse_checksum_sum_attribute() {
        let input = r#"
            #[send]
            struct ChecksumPacket {
                data: u16 = 0x1234,
                #[checksum(sum)]
                checksum: u8,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("ChecksumPacket").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::Checksum(ChecksumAlgorithm::Sum)));
    }

    #[test]
    fn test_parse_bits_attribute() {
        let input = r#"
            #[send]
            struct StatusRegister {
                #[bits(0, 3)]
                code: u8 = 5,
                #[bit(4)]
                active: bool = true,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("StatusRegister").unwrap();
        assert!(def.fields[0]
            .attributes
            .contains(&FieldAttribute::Bits(0, 3)));
        assert!(def.fields[1].attributes.contains(&FieldAttribute::Bit(4)));
    }

    #[test]
    fn test_parse_if_attribute() {
        let input = r#"
            #[send]
            struct ConditionalData {
                has_data: bool = true,
                #[if(has_data)]
                data: u32 = 100,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("ConditionalData").unwrap();
        assert!(def.fields[1]
            .attributes
            .contains(&FieldAttribute::If("has_data".to_string())));
    }

    #[test]
    fn test_parse_multiple_field_attributes() {
        let input = r#"
            #[send]
            struct ComplexPacket {
                count: u8 = 3,
                #[if(has_data)]
                #[len_ref(count)]
                data: Vec<u8> = [1, 2, 3],
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("ComplexPacket").unwrap();
        assert_eq!(def.fields[1].attributes.len(), 2);
    }

    #[test]
    fn test_parse_struct_endian_attribute() {
        let input = r#"
            #[endian(little)]
            struct LittleEndianStruct {
                value: u32 = 0x12345678,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("LittleEndianStruct").unwrap();
        assert_eq!(def.endian, Some(Endianness::Little));
    }

    #[test]
    fn test_parse_doc_comment() {
        let input = r#"
            /// This is a sensor data packet
            /// Used for temperature readings
            #[send]
            struct SensorData {
                /// Temperature in Celsius
                temp: f32 = 25.0,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();
        let def = schema.get_struct("SensorData").unwrap();
        assert!(def.doc.is_some());
        assert!(def.doc.as_ref().unwrap().contains("sensor data packet"));
        assert!(def.doc.as_ref().unwrap().contains("temperature readings"));

        // 检查字段文档
        assert_eq!(def.fields.len(), 1);
        assert!(def.fields[0].doc.is_some());
        assert!(def.fields[0]
            .doc
            .as_ref()
            .unwrap()
            .contains("Temperature in Celsius"));
    }

    #[test]
    fn test_parse_import() {
        let input = r#"
            import("common/types.pkt")
            import("network/packet.pkt")

            #[send]
            struct MyPacket {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let schema = result.unwrap();

        // 检查导入语句
        assert_eq!(schema.imports.len(), 2);
        assert_eq!(schema.imports[0].path, "common/types.pkt");
        assert_eq!(schema.imports[1].path, "network/packet.pkt");
    }

    #[test]
    fn test_parse_import_with_doc() {
        let input = r#"
            /// Common types for sensors
            import("common/sensor.pkt")

            #[send]
            struct SensorData {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        // 检查带文档的导入
        assert_eq!(schema.imports.len(), 1);
        assert_eq!(schema.imports[0].path, "common/sensor.pkt");
        assert!(schema.imports[0].doc.is_some());
        assert!(schema.imports[0]
            .doc
            .as_ref()
            .unwrap()
            .contains("Common types for sensors"));
    }

    #[test]
    fn test_parse_import_only() {
        // 只包含import的schema
        let input = r#"
            import("a.pkt")
            import("b.pkt")
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        assert_eq!(schema.imports.len(), 2);
        assert!(schema.structs.is_empty());
    }

    #[test]
    fn test_parse_file_attribute_version() {
        let input = r#"
            #![version("1.0.0")]

            #[send]
            struct TestPacket {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok(), "parse failed: {:?}", result.err());
        let schema = result.unwrap();

        assert_eq!(schema.file_attributes.len(), 1);
        match &schema.file_attributes[0] {
            FileAttribute::Version(v) => assert_eq!(v, "1.0.0"),
            _ => panic!("Expected Version attribute"),
        }
    }

    #[test]
    fn test_parse_file_attribute_endian_big() {
        let input = r#"
            #![endian(big)]

            #[send]
            struct TestPacket {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        assert_eq!(schema.file_attributes.len(), 1);
        match &schema.file_attributes[0] {
            FileAttribute::Endian(e) => assert_eq!(*e, Endianness::Big),
            _ => panic!("Expected Endian attribute"),
        }
    }

    #[test]
    fn test_parse_file_attribute_endian_little() {
        let input = r#"
            #![endian(little)]

            #[send]
            struct TestPacket {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        assert_eq!(schema.file_attributes.len(), 1);
        match &schema.file_attributes[0] {
            FileAttribute::Endian(e) => assert_eq!(*e, Endianness::Little),
            _ => panic!("Expected Endian attribute"),
        }
    }

    #[test]
    fn test_parse_file_attribute_import_path() {
        let input = r#"
            #![import_path("protocols/")]

            #[send]
            struct TestPacket {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        assert_eq!(schema.file_attributes.len(), 1);
        match &schema.file_attributes[0] {
            FileAttribute::ImportPath(p) => assert_eq!(p, "protocols/"),
            _ => panic!("Expected ImportPath attribute"),
        }
    }

    #[test]
    fn test_parse_file_attribute_doc() {
        let input = r#"
            #![doc("Sensor protocol definitions")]

            #[send]
            struct TestPacket {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        assert_eq!(schema.file_attributes.len(), 1);
        match &schema.file_attributes[0] {
            FileAttribute::Doc(d) => assert_eq!(d, "Sensor protocol definitions"),
            _ => panic!("Expected Doc attribute"),
        }
    }

    #[test]
    fn test_parse_multiple_file_attributes() {
        let input = r#"
            #![version("2.1.0")]
            #![endian(little)]

            #[send]
            struct TestPacket {
                data: u32 = 42,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        assert_eq!(schema.file_attributes.len(), 2);
        match &schema.file_attributes[0] {
            FileAttribute::Version(v) => assert_eq!(v, "2.1.0"),
            _ => panic!("Expected Version attribute"),
        }
        match &schema.file_attributes[1] {
            FileAttribute::Endian(e) => assert_eq!(*e, Endianness::Little),
            _ => panic!("Expected Endian attribute"),
        }
    }

    #[test]
    fn test_parse_full_schema_with_all_features() {
        let input = r#"
            #![version("1.0.0")]
            #![endian(big)]

            /// Common types
            import("common/types.pkt")

            /// Sensor data packet
            /// Used for temperature readings
            #[send]
            struct SensorData {
                /// Temperature in Celsius
                temp: f32 = 25.0,
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        // 检查文件级属性
        assert_eq!(schema.file_attributes.len(), 2);

        // 检查导入
        assert_eq!(schema.imports.len(), 1);
        assert_eq!(schema.imports[0].path, "common/types.pkt");

        // 检查结构体
        assert_eq!(schema.structs.len(), 1);
        let def = schema.get_struct("SensorData").unwrap();
        assert!(def.doc.as_ref().unwrap().contains("Sensor data packet"));
        assert_eq!(def.fields[0].name, "temp");
    }

    #[test]
    fn test_parse_prefix_disable_attribute() {
        let input = r#"
            #![prefix(disable)]

            #[send]
            struct ManualLenPacket {
                #[auto(data)]
                count: u8,
                data: Vec<u8> = [10, 20, 30],
            }
        "#;
        let (tokens, spans) = lex(input);
        let result = parse_schema_tokens(&tokens, &spans);
        assert!(result.is_ok());
        let schema = result.unwrap();

        assert_eq!(schema.file_attributes.len(), 1);
        match &schema.file_attributes[0] {
            FileAttribute::PrefixDisabled => {}
            _ => panic!("Expected PrefixDisabled attribute"),
        }
        assert!(!schema.is_prefix_enabled());
    }
}
