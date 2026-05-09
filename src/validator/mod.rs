//! Validator 模块
//!
//! 提供 Schema 的验证功能。

use crate::ast::{Direction, Field, FieldAttribute, Schema, StructDef, Type, Value};
use crate::error::{CoreError, Result};
use std::collections::HashSet;

/// 验证 Schema
///
/// # Arguments
///
/// * `schema` - 要验证的 Schema
///
/// # Returns
///
/// 成功返回 `Ok(())`，失败返回验证错误
///
/// # 验证项
///
/// - 结构体名称唯一
/// - 字段名称唯一
/// - 类型引用有效
/// - 值类型匹配
/// - #[remaining] 约束检查
/// - #[if] 条件字段引用检查
/// - #[`len_ref`] 引用字段检查
/// - #[auto(field)] 引用字段检查
pub fn validate_schema(schema: &Schema) -> Result<()> {
    // 1. 检查结构体名称唯一性
    let mut struct_names = HashSet::new();
    for struct_def in &schema.structs {
        if !struct_names.insert(&struct_def.name) {
            return Err(CoreError::validation(
                "E002",
                format!("duplicate struct name: '{}'", struct_def.name),
            ));
        }
    }

    // 2. 验证每个结构体
    for struct_def in &schema.structs {
        validate_struct(struct_def, schema)?;
    }

    Ok(())
}

/// 验证结构体
fn validate_struct(struct_def: &StructDef, schema: &Schema) -> Result<()> {
    // 检查结构体名称
    if struct_def.name.is_empty() {
        return Err(CoreError::validation("E001", "struct name cannot be empty"));
    }

    // 检查字段
    let mut field_names = HashSet::new();
    let mut remaining_count = 0;
    let mut remaining_position = None;

    for (idx, field) in struct_def.fields.iter().enumerate() {
        // 检查字段名称唯一性
        if !field_names.insert(&field.name) {
            return Err(CoreError::validation(
                "E003",
                format!(
                    "duplicate field name '{}' in struct '{}'",
                    field.name, struct_def.name
                ),
            ));
        }

        // 验证字段
        validate_field(field, schema, struct_def)?;

        // 检查 remaining 属性
        if has_remaining_attribute(&field.attributes) {
            remaining_count += 1;
            remaining_position = Some(idx);
        }
    }

    // 检查 remaining 约束
    if remaining_count > 0 {
        // E014: 多个 remaining 字段
        if remaining_count > 1 {
            return Err(CoreError::validation(
                "E014",
                format!(
                    "struct '{}' has multiple #[remaining] fields, only one allowed",
                    struct_def.name
                ),
            ));
        }

        // E012: remaining 仅用于 receive
        if struct_def.direction != Some(Direction::Receive) {
            return Err(CoreError::validation(
                "E012",
                format!(
                    "struct '{}' has #[remaining] field but is not a receive struct",
                    struct_def.name
                ),
            ));
        }

        // E013: remaining 必须是最后一个字段
        if let Some(pos) = remaining_position {
            if pos != struct_def.fields.len() - 1 {
                return Err(CoreError::validation(
                    "E013",
                    format!(
                        "#[remaining] field in struct '{}' must be the last field",
                        struct_def.name
                    ),
                ));
            }
        }

        // E011: remaining 类型检查
        if let Some(field) = struct_def.fields.last() {
            if has_remaining_attribute(&field.attributes) {
                match &field.ty {
                    Type::Bytes | Type::String | Type::Vec(_) => {}
                    _ => {
                        return Err(CoreError::validation(
                            "E011",
                            format!(
                                "#[remaining] field '{}' in struct '{}' must be Bytes, String, or Vec<T>",
                                field.name, struct_def.name
                            ),
                        ));
                    }
                }
            }
        }
    }

    // 检查 send 结构体是否有值
    if struct_def.direction == Some(Direction::Send) {
        for field in &struct_def.fields {
            if field.value.is_none() && !has_auto_attribute(&field.attributes) {
                return Err(CoreError::validation(
                    "E004",
                    format!(
                        "field '{}' in send struct '{}' must have a value or be auto",
                        field.name, struct_def.name
                    ),
                ));
            }
        }
    }

    Ok(())
}

/// 验证字段
fn validate_field(field: &Field, schema: &Schema, struct_def: &StructDef) -> Result<()> {
    // 检查字段名称
    if field.name.is_empty() {
        return Err(CoreError::validation(
            "E005",
            format!("field name cannot be empty in struct '{}'", struct_def.name),
        ));
    }

    // 验证类型
    validate_type(&field.ty, schema, struct_def)?;

    // 验证值类型匹配
    if let Some(ref value) = field.value {
        if !value_matches_type(value, &field.ty) {
            return Err(CoreError::validation(
                "E006",
                format!(
                    "field '{}' value type does not match declared type '{:?}'",
                    field.name, field.ty
                ),
            ));
        }
    }

    // 验证字段属性
    validate_field_attributes(field, struct_def)?;

    Ok(())
}

/// 验证字段属性
fn validate_field_attributes(field: &Field, struct_def: &StructDef) -> Result<()> {
    for attr in &field.attributes {
        match attr {
            FieldAttribute::If(condition) => {
                // E006: 条件字段引用检查
                let cond_field = struct_def.fields.iter().find(|f| f.name == *condition);
                match cond_field {
                    Some(f) => {
                        if f.ty != Type::Bool {
                            return Err(CoreError::validation(
                                "E006",
                                format!(
                                    "#[if({})] in struct '{}' references field '{}' which is not bool type",
                                    condition, struct_def.name, condition
                                ),
                            ));
                        }
                    }
                    None => {
                        return Err(CoreError::validation(
                            "E006",
                            format!(
                                "#[if({})] in struct '{}' references non-existent field '{}'",
                                condition, struct_def.name, condition
                            ),
                        ));
                    }
                }
            }
            FieldAttribute::LenRef(ref_field) => {
                // E007: len_ref 引用检查
                let referenced = struct_def.fields.iter().find(|f| f.name == *ref_field);
                match referenced {
                    Some(f) => {
                        // 检查引用的字段是否为整数类型
                        match f.ty {
                            Type::U8
                            | Type::U16
                            | Type::U32
                            | Type::U64
                            | Type::I8
                            | Type::I16
                            | Type::I32
                            | Type::I64 => {}
                            _ => {
                                return Err(CoreError::validation(
                                    "E007",
                                    format!(
                                        "#[len_ref({})] in struct '{}' references field '{}' which is not an integer type",
                                        ref_field, struct_def.name, ref_field
                                    ),
                                ));
                            }
                        }
                    }
                    None => {
                        return Err(CoreError::validation(
                            "E007",
                            format!(
                                "#[len_ref({})] in struct '{}' references non-existent field '{}'",
                                ref_field, struct_def.name, ref_field
                            ),
                        ));
                    }
                }
            }
            FieldAttribute::Auto(Some(ref_field)) => {
                // E015: auto(field) 引用检查
                let referenced = struct_def.fields.iter().find(|f| f.name == *ref_field);
                if referenced.is_none() {
                    return Err(CoreError::validation(
                        "E015",
                        format!(
                            "#[auto({})] in struct '{}' references non-existent field '{}'",
                            ref_field, struct_def.name, ref_field
                        ),
                    ));
                }
            }
            FieldAttribute::Checksum(_) => {
                // E008: 校验和字段类型检查
                match field.ty {
                    Type::U8 | Type::U16 | Type::U32 | Type::U64 => {}
                    _ => {
                        return Err(CoreError::validation(
                            "E008",
                            format!(
                                "#[checksum] field '{}' in struct '{}' must be an unsigned integer type",
                                field.name, struct_def.name
                            ),
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// 验证类型
fn validate_type(ty: &Type, schema: &Schema, struct_def: &StructDef) -> Result<()> {
    match ty {
        Type::Custom(name) => {
            // 检查自定义类型是否存在
            #[allow(clippy::collapsible_match)]
            if schema.get_struct(name).is_none() {
                return Err(CoreError::validation(
                    "E009",
                    format!(
                        "unknown type '{}' in field of struct '{}'",
                        name, struct_def.name
                    ),
                ));
            }
        }
        Type::Vec(inner) => {
            validate_type(inner, schema, struct_def)?;
        }
        Type::Array(inner, size) => {
            if *size == 0 {
                return Err(CoreError::validation(
                    "E010",
                    format!("array size cannot be zero in struct '{}'", struct_def.name),
                ));
            }
            validate_type(inner, schema, struct_def)?;
        }
        _ => {
            // 基础类型总是有效的
        }
    }
    Ok(())
}

/// 检查值是否匹配类型
fn value_matches_type(value: &Value, ty: &Type) -> bool {
    match (value, ty) {
        (Value::Integer(_), Type::U8) => true,
        (Value::Integer(_), Type::U16) => true,
        (Value::Integer(_), Type::U32) => true,
        (Value::Integer(_), Type::U64) => true,
        (Value::Integer(_), Type::U128) => true,
        (Value::Integer(_), Type::I8) => true,
        (Value::Integer(_), Type::I16) => true,
        (Value::Integer(_), Type::I32) => true,
        (Value::Integer(_), Type::I64) => true,
        (Value::Integer(_), Type::I128) => true,
        // 十六进制整数也可以作为 Bytes 的值
        (Value::Integer(_), Type::Bytes) => true,
        (Value::Float(_), Type::F32) => true,
        (Value::Float(_), Type::F64) => true,
        (Value::Bool(_), Type::Bool) => true,
        (Value::String(_), Type::String) => true,
        (Value::Bytes(_), Type::Bytes) => true,
        (Value::Array(arr), Type::Vec(inner)) => arr.iter().all(|v| value_matches_type(v, inner)),
        (Value::Array(arr), Type::Array(inner, size)) => {
            arr.len() == *size && arr.iter().all(|v| value_matches_type(v, inner))
        }
        // 结构体初始化值匹配自定义类型
        (Value::Struct(_, _), Type::Custom(_)) => true,
        _ => false,
    }
}

/// 检查是否有 auto 属性
fn has_auto_attribute(attrs: &[crate::ast::FieldAttribute]) -> bool {
    attrs
        .iter()
        .any(|attr| matches!(attr, crate::ast::FieldAttribute::Auto(_)))
}

/// 检查是否有 remaining 属性
fn has_remaining_attribute(attrs: &[crate::ast::FieldAttribute]) -> bool {
    attrs
        .iter()
        .any(|attr| matches!(attr, crate::ast::FieldAttribute::Remaining))
}

/// 验证结果
#[derive(Debug)]
pub struct ValidationResult {
    /// 是否有效
    pub is_valid: bool,
    /// 错误列表
    pub errors: Vec<ValidationError>,
}

/// 验证错误
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 相关结构体
    pub struct_name: Option<String>,
    /// 相关字段
    pub field_name: Option<String>,
}

impl ValidationResult {
    /// 创建成功的验证结果
    #[must_use]
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: vec![],
        }
    }

    /// 添加错误
    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Field, StructDef, Type, Value};

    fn create_test_schema() -> Schema {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![Field {
                name: "value".to_string(),
                ty: Type::U32,
                value: Some(Value::Integer(42)),
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });
        schema
    }

    #[test]
    fn test_validate_valid_schema() {
        let schema = create_test_schema();
        let result = validate_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_duplicate_struct_name() {
        let mut schema = Schema::new();
        let struct_def = StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![],
            doc: None,
            endian: None,
        };
        schema.add_struct(struct_def.clone());
        schema.add_struct(struct_def);

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate"));
    }

    #[test]
    fn test_validate_duplicate_field_name() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![
                Field {
                    name: "value".to_string(),
                    ty: Type::U32,
                    value: Some(Value::Integer(1)),
                    attributes: vec![],
                    doc: None,
                },
                Field {
                    name: "value".to_string(),
                    ty: Type::U32,
                    value: Some(Value::Integer(2)),
                    attributes: vec![],
                    doc: None,
                },
            ],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate field"));
    }

    #[test]
    fn test_validate_unknown_type() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![Field {
                name: "value".to_string(),
                ty: Type::Custom("Unknown".to_string()),
                value: None,
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown type"));
    }

    #[test]
    fn test_validate_type_mismatch() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![Field {
                name: "value".to_string(),
                ty: Type::Bool,
                value: Some(Value::Integer(42)),
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("type"));
    }

    #[test]
    fn test_validate_empty_struct_name() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "".to_string(),
            direction: None,
            fields: vec![],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_field_name() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![Field {
                name: "".to_string(),
                ty: Type::U32,
                value: None,
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_send_struct_missing_value() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Send),
            fields: vec![Field {
                name: "value".to_string(),
                ty: Type::U32,
                value: None,
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must have a value"));
    }

    #[test]
    fn test_validate_zero_size_array() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![Field {
                name: "arr".to_string(),
                ty: Type::Array(Box::new(Type::U8), 0),
                value: None,
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be zero"));
    }

    #[test]
    fn test_validate_nested_vec() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: None,
            fields: vec![Field {
                name: "nested".to_string(),
                ty: Type::Vec(Box::new(Type::Vec(Box::new(Type::U32)))),
                value: None,
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_custom_type() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Inner".to_string(),
            direction: None,
            fields: vec![],
            doc: None,
            endian: None,
        });
        schema.add_struct(StructDef {
            name: "Outer".to_string(),
            direction: None,
            fields: vec![Field {
                name: "inner".to_string(),
                ty: Type::Custom("Inner".to_string()),
                value: None,
                attributes: vec![],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_ok());
    }

    #[test]
    fn test_value_matches_type() {
        assert!(super::value_matches_type(&Value::Integer(42), &Type::U32));
        assert!(super::value_matches_type(&Value::Bool(true), &Type::Bool));
        assert!(super::value_matches_type(&Value::Float(3.14), &Type::F64));
        assert!(!super::value_matches_type(&Value::Integer(42), &Type::Bool));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::success();
        assert!(result.is_valid);

        result.add_error(ValidationError {
            code: "E001".to_string(),
            message: "test".to_string(),
            struct_name: None,
            field_name: None,
        });
        assert!(!result.is_valid);
    }

    // ========== 新增约束测试 ==========

    #[test]
    fn test_remaining_in_send_struct() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Send),
            fields: vec![Field {
                name: "data".to_string(),
                ty: Type::Bytes,
                value: Some(Value::Bytes(vec![1, 2, 3])),
                attributes: vec![FieldAttribute::Remaining],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E012"));
    }

    #[test]
    fn test_remaining_not_last_field() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Receive),
            fields: vec![
                Field {
                    name: "data".to_string(),
                    ty: Type::Bytes,
                    value: None,
                    attributes: vec![FieldAttribute::Remaining],
                    doc: None,
                },
                Field {
                    name: "extra".to_string(),
                    ty: Type::U32,
                    value: None,
                    attributes: vec![],
                    doc: None,
                },
            ],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E013"));
    }

    #[test]
    fn test_multiple_remaining_fields() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Receive),
            fields: vec![
                Field {
                    name: "data1".to_string(),
                    ty: Type::Bytes,
                    value: None,
                    attributes: vec![FieldAttribute::Remaining],
                    doc: None,
                },
                Field {
                    name: "data2".to_string(),
                    ty: Type::Bytes,
                    value: None,
                    attributes: vec![FieldAttribute::Remaining],
                    doc: None,
                },
            ],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E014"));
    }

    #[test]
    fn test_remaining_invalid_type() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Receive),
            fields: vec![Field {
                name: "data".to_string(),
                ty: Type::U32,
                value: None,
                attributes: vec![FieldAttribute::Remaining],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E011"));
    }

    #[test]
    fn test_if_condition_not_bool() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Send),
            fields: vec![
                Field {
                    name: "flag".to_string(),
                    ty: Type::U32,
                    value: Some(Value::Integer(1)),
                    attributes: vec![],
                    doc: None,
                },
                Field {
                    name: "data".to_string(),
                    ty: Type::U32,
                    value: Some(Value::Integer(42)),
                    attributes: vec![FieldAttribute::If("flag".to_string())],
                    doc: None,
                },
            ],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E006"));
    }

    #[test]
    fn test_if_condition_nonexistent() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Send),
            fields: vec![Field {
                name: "data".to_string(),
                ty: Type::U32,
                value: Some(Value::Integer(42)),
                attributes: vec![FieldAttribute::If("nonexistent".to_string())],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E006"));
    }

    #[test]
    fn test_checksum_invalid_type() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Send),
            fields: vec![Field {
                name: "crc".to_string(),
                ty: Type::F32,
                value: Some(Value::Float(0.0)),
                attributes: vec![FieldAttribute::Checksum(
                    crate::ast::ChecksumAlgorithm::Crc8,
                )],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E008"));
    }

    #[test]
    fn test_auto_field_reference_nonexistent() {
        let mut schema = Schema::new();
        schema.add_struct(StructDef {
            name: "Test".to_string(),
            direction: Some(Direction::Send),
            fields: vec![Field {
                name: "count".to_string(),
                ty: Type::U8,
                value: None,
                attributes: vec![FieldAttribute::Auto(Some("nonexistent".to_string()))],
                doc: None,
            }],
            doc: None,
            endian: None,
        });

        let result = validate_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("E015"));
    }
}
