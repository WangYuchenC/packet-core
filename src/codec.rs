//! Codec 模块
//!
//! 提供数据包的编码和解码功能。

use crate::ast::{Direction, Endianness, FieldAttribute, Schema, StructDef, Type, Value};
use crate::error::{CoreError, Result};

type FieldOffsets = std::collections::HashMap<String, (usize, usize)>;

/// 编解码器
pub struct Codec<'a> {
    schema: &'a Schema,
    struct_def: &'a StructDef,
    /// 默认字节序
    default_endian: Endianness,
}

/// 解码后的结构体
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedStruct {
    /// 结构体名称
    pub name: String,
    /// 字段列表 (名称, 值)
    pub fields: Vec<(String, DecodedValue)>,
}

/// 解码后的值
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedValue {
    /// 8位无符号整数
    U8(u8),
    /// 16位无符号整数
    U16(u16),
    /// 32位无符号整数
    U32(u32),
    /// 64位无符号整数
    U64(u64),
    /// 128位无符号整数
    U128(u128),
    /// 8位有符号整数
    I8(i8),
    /// 16位有符号整数
    I16(i16),
    /// 32位有符号整数
    I32(i32),
    /// 64位有符号整数
    I64(i64),
    /// 128位有符号整数
    I128(i128),
    /// 32位浮点数
    F32(f32),
    /// 64位浮点数
    F64(f64),
    /// 布尔值
    Bool(bool),
    /// UTF-8 字符串
    String(String),
    /// 字节数组
    Bytes(Vec<u8>),
    /// 数组
    Vec(Vec<DecodedValue>),
    /// 嵌套结构体
    Struct(String, Vec<(String, DecodedValue)>),
}

impl<'a> Codec<'a> {
    /// 编译 Codec
    ///
    /// # Arguments
    ///
    /// * `schema` - Schema AST
    /// * `struct_name` - 结构体名称
    ///
    /// # Returns
    ///
    /// 成功返回 `Codec` 实例
    ///
    /// # Errors
    ///
    /// - `CoreError::Validation`: 结构体不存在
    ///
    /// # Examples
    ///
    /// ```
    /// use packet_core::{parse_schema, Codec};
    ///
    /// let schema = parse_schema(r#"
    ///     #[send]
    ///     struct Test { value: u32 = 42, }
    /// "#).unwrap();
    ///
    /// let codec = Codec::compile(&schema, "Test").unwrap();
    /// ```
    pub fn compile(schema: &'a Schema, struct_name: &str) -> Result<Self> {
        let struct_def = schema.get_struct(struct_name).ok_or_else(|| {
            CoreError::validation("E001", format!("struct '{struct_name}' not found"))
        })?;

        let default_endian = schema.default_endian();

        Ok(Self {
            schema,
            struct_def,
            default_endian,
        })
    }

    /// 使用指定字节序编译 Codec
    pub fn compile_with_endian(
        schema: &'a Schema,
        struct_name: &str,
        endian: Endianness,
    ) -> Result<Self> {
        let struct_def = schema.get_struct(struct_name).ok_or_else(|| {
            CoreError::validation("E001", format!("struct '{struct_name}' not found"))
        })?;

        Ok(Self {
            schema,
            struct_def,
            default_endian: endian,
        })
    }

    /// 编码为字节
    ///
    /// # Preconditions
    /// - 结构体必须是 `#[send]` 类型
    /// - 所有字段必须有值（除 #[auto], #[remaining] 外）
    ///
    /// # Returns
    ///
    /// 成功返回编码后的字节数组
    ///
    /// # Errors
    /// - `CoreError::Codec`: 结构体不是 send 类型
    /// - `CoreError::Codec`: 字段缺少值
    pub fn encode(&self) -> Result<Vec<u8>> {
        // 检查是否为 send 类型
        if self.struct_def.direction != Some(Direction::Send) {
            return Err(CoreError::codec(
                "encode",
                format!("struct '{}' is not a send type", self.struct_def.name),
            ));
        }

        // 构建字段值映射（从 struct_def.fields 中提取默认值）
        let field_values: std::collections::HashMap<String, Value> = self
            .struct_def
            .fields
            .iter()
            .filter_map(|f| f.value.as_ref().map(|v| (f.name.clone(), v.clone())))
            .collect();

        let endian = self.get_struct_endian();

        // 第一阶段：完整编码，收集所有 auto 字段的位置
        let (mut buffer, _field_offsets, auto_field_infos) = encode_struct_with_auto_tracking(
            self.struct_def,
            &field_values,
            endian,
            self.schema,
            0, // 基础偏移量
        )?;

        // 第二阶段：回填所有 auto 字段
        let total_size = buffer.len();
        for (offset, field, struct_def_for_field) in auto_field_infos {
            Self::fill_auto_field(
                &mut buffer,
                offset,
                &field,
                &struct_def_for_field,
                endian,
                total_size,
                &field_values,
            )?;
        }

        Ok(buffer)
    }

    /// 回填单个 auto 字段
    fn fill_auto_field(
        buffer: &mut [u8],
        offset: usize,
        field: &crate::ast::Field,
        _struct_def: &StructDef,
        endian: Endianness,
        total_size: usize,
        _field_values: &std::collections::HashMap<String, Value>,
    ) -> Result<()> {
        // 找到 auto 字段的属性
        let auto_attr = field
            .attributes
            .iter()
            .find_map(|attr| match attr {
                FieldAttribute::Auto(ref_field) => Some(ref_field.clone()),
                _ => None,
            })
            .ok_or_else(|| CoreError::codec("encode", "auto attribute not found"))?;

        let length = match auto_attr {
            None => {
                // #[auto] - 计算整个结构体的总字节数
                total_size as u64
            }
            Some(ref_field_name) => {
                // #[auto(field)] - 计算指定字段的元素个数或字节数
                // 需要从 field_values 中获取引用字段的值
                let ref_field = _struct_def
                    .fields
                    .iter()
                    .find(|f| f.name == ref_field_name);
                if let Some(ref_value) = _field_values.get(ref_field_name.as_str()) {
                    match ref_value {
                        Value::Array(arr) => arr.len() as u64,
                        Value::Bytes(bytes) => bytes.len() as u64,
                        Value::String(s) => s.len() as u64,
                        // 处理十六进制整数作为 Bytes（如 Bytes = 0xBEAD）
                        Value::Integer(n)
                            if ref_field.map_or(false, |f| f.ty == Type::Bytes) =>
                        {
                            integer_to_byte_len(*n)
                        }
                        _ => 1,
                    }
                } else {
                    // 在 struct_def 中查找字段默认值
                    if let Some(ref_field) = ref_field {
                        if let Some(ref_value) = &ref_field.value {
                            match ref_value {
                                Value::Array(arr) => arr.len() as u64,
                                Value::Bytes(bytes) => bytes.len() as u64,
                                Value::String(s) => s.len() as u64,
                                // 处理十六进制整数作为 Bytes（如 Bytes = 0xBEAD）
                                Value::Integer(n)
                                    if ref_field.ty == Type::Bytes =>
                                {
                                    integer_to_byte_len(*n)
                                }
                                _ => 1,
                            }
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                }
            }
        };

        // 获取字段字节序
        let field_endian = field
            .attributes
            .iter()
            .find_map(|attr| match attr {
                FieldAttribute::Endian(e) => Some(*e),
                _ => None,
            })
            .unwrap_or(endian);

        // 写入长度值
        match field.ty {
            Type::U8 => buffer[offset] = length as u8,
            Type::U16 => {
                let bytes = match field_endian {
                    Endianness::Big => (length as u16).to_be_bytes(),
                    Endianness::Little => (length as u16).to_le_bytes(),
                };
                buffer[offset..offset + 2].copy_from_slice(&bytes);
            }
            Type::U32 => {
                let bytes = match field_endian {
                    Endianness::Big => (length as u32).to_be_bytes(),
                    Endianness::Little => (length as u32).to_le_bytes(),
                };
                buffer[offset..offset + 4].copy_from_slice(&bytes);
            }
            Type::U64 => {
                let bytes = match field_endian {
                    Endianness::Big => length.to_be_bytes(),
                    Endianness::Little => length.to_le_bytes(),
                };
                buffer[offset..offset + 8].copy_from_slice(&bytes);
            }
            _ => {
                return Err(CoreError::codec(
                    "encode",
                    format!(
                        "auto field '{}' must be an unsigned integer type",
                        field.name
                    ),
                ));
            }
        }

        Ok(())
    }

    /// 解码字节
    ///
    /// # Preconditions
    /// - 结构体必须是 `#[receive]` 类型
    ///
    /// # Arguments
    ///
    /// * `data` - 要解码的字节数组（大端序）
    ///
    /// # Returns
    ///
    /// 成功返回 `DecodedStruct`
    ///
    /// # Errors
    /// - `CoreError::Codec`: 结构体不是 receive 类型
    /// - `CoreError::Codec`: 数据不足或格式错误
    pub fn decode(&self, data: &[u8]) -> Result<DecodedStruct> {
        // 检查是否为 receive 类型
        if self.struct_def.direction != Some(Direction::Receive) {
            return Err(CoreError::codec(
                "decode",
                format!("struct '{}' is not a receive type", self.struct_def.name),
            ));
        }

        let mut offset = 0;
        let mut fields = Vec::with_capacity(self.struct_def.fields.len());
        let mut processed_bitfields: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for field in &self.struct_def.fields {
            // 跳过已处理的位域字段
            if processed_bitfields.contains(&field.name) {
                continue;
            }

            let endian = self.get_field_endian(field);

            // 检查是否为位域字段
            if get_bits_attribute(&field.attributes).is_some()
                || get_bit_attribute(&field.attributes).is_some()
            {
                // 按位位置范围分组处理位域
                let mut bitfield_groups: std::collections::HashMap<usize, Vec<&crate::ast::Field>> =
                    std::collections::HashMap::new();
                for f in &self.struct_def.fields {
                    if let Some((_start, end)) = get_bits_attribute(&f.attributes) {
                        let max_bit = end;
                        let storage_key = if max_bit <= 7 {
                            8
                        } else if max_bit <= 15 {
                            16
                        } else if max_bit <= 31 {
                            32
                        } else {
                            64
                        };
                        bitfield_groups.entry(storage_key).or_default().push(f);
                    } else if let Some(pos) = get_bit_attribute(&f.attributes) {
                        let storage_key = if pos <= 7 {
                            8
                        } else if pos <= 15 {
                            16
                        } else if pos <= 31 {
                            32
                        } else {
                            64
                        };
                        bitfield_groups.entry(storage_key).or_default().push(f);
                    }
                }

                // 处理每个位域组
                for (storage_bits, group_fields) in bitfield_groups {
                    if group_fields.is_empty() {
                        continue;
                    }

                    let base_size = storage_bits / 8;

                    check_buffer_size(data, offset, base_size)?;

                    // 读取基础值
                    let raw_value = match storage_bits {
                        8 => u64::from(data[offset]),
                        16 => {
                            let bytes = [data[offset], data[offset + 1]];
                            match endian {
                                Endianness::Big => u64::from(u16::from_be_bytes(bytes)),
                                Endianness::Little => u64::from(u16::from_le_bytes(bytes)),
                            }
                        }
                        32 => {
                            let bytes = [
                                data[offset],
                                data[offset + 1],
                                data[offset + 2],
                                data[offset + 3],
                            ];
                            match endian {
                                Endianness::Big => u64::from(u32::from_be_bytes(bytes)),
                                Endianness::Little => u64::from(u32::from_le_bytes(bytes)),
                            }
                        }
                        64 => {
                            let bytes = [
                                data[offset],
                                data[offset + 1],
                                data[offset + 2],
                                data[offset + 3],
                                data[offset + 4],
                                data[offset + 5],
                                data[offset + 6],
                                data[offset + 7],
                            ];
                            match endian {
                                Endianness::Big => u64::from_be_bytes(bytes),
                                Endianness::Little => u64::from_le_bytes(bytes),
                            }
                        }
                        _ => 0,
                    };

                    // 提取每个位域字段的值
                    for bf in &group_fields {
                        if let Some((start, end)) = get_bits_attribute(&bf.attributes) {
                            let value = extract_bits(raw_value, start, end);
                            fields.push((bf.name.clone(), DecodedValue::U64(value)));
                        } else if let Some(pos) = get_bit_attribute(&bf.attributes) {
                            let value = extract_bits(raw_value, pos, pos);
                            fields.push((bf.name.clone(), DecodedValue::Bool(value != 0)));
                        }
                        processed_bitfields.insert(bf.name.clone());
                    }

                    offset += base_size;
                }
                continue;
            }

            // 检查是否为 auto 字段（receive 中作为普通字段解码）
            if has_auto_attribute(&field.attributes) {
                let (value, consumed) =
                    decode_value_with_endian(data, offset, &field.ty, self.schema, endian)?;
                fields.push((field.name.clone(), value));
                offset += consumed;
                continue;
            }

            // 检查是否为 len_ref 字段（receive 中作为普通字段解码）
            if get_len_ref_attribute(&field.attributes).is_some() {
                let prefix_enabled = self.schema.is_prefix_enabled();

                if !prefix_enabled && is_variable_length_type(&field.ty) {
                    // prefix 禁用 + len_ref：手动解码变长字段
                    let ref_field_name = get_len_ref_attribute(&field.attributes).unwrap();
                    let count = get_decoded_field_value_as_usize(&fields, ref_field_name)
                        .ok_or_else(|| CoreError::codec(
                            "decode",
                            format!(
                                "len_ref references field '{}' which is not yet decoded or not an integer",
                                ref_field_name
                            ),
                        ))?;

                    match &field.ty {
                        Type::Vec(inner) => {
                            let mut values = Vec::new();
                            let mut cur = offset;
                            for _ in 0..count {
                                let (val, consumed) = decode_value_with_endian(
                                    data, cur, inner, self.schema, endian,
                                )?;
                                values.push(val);
                                cur += consumed;
                            }
                            fields.push((field.name.clone(), DecodedValue::Vec(values)));
                            offset = cur;
                        }
                        Type::Bytes => {
                            check_buffer_size(data, offset, count)?;
                            let bytes_data = data[offset..offset + count].to_vec();
                            fields.push((field.name.clone(), DecodedValue::Bytes(bytes_data)));
                            offset += count;
                        }
                        Type::String => {
                            check_buffer_size(data, offset, count)?;
                            let s = String::from_utf8(data[offset..offset + count].to_vec())
                                .map_err(|_| CoreError::codec(
                                    "decode",
                                    format!("len_ref field '{}' contains invalid UTF-8", field.name),
                                ))?;
                            fields.push((field.name.clone(), DecodedValue::String(s)));
                            offset += count;
                        }
                        _ => {
                            return Err(CoreError::codec(
                                "decode",
                                format!(
                                    "len_ref on non-variable type '{:?}' with prefix disabled is not supported",
                                    field.ty
                                ),
                            ));
                        }
                    }
                } else {
                    let (value, consumed) =
                        decode_value_with_endian(data, offset, &field.ty, self.schema, endian)?;
                    fields.push((field.name.clone(), value));
                    offset += consumed;
                }
                continue;
            }

            // 检查是否为 remaining 字段
            if has_remaining_attribute(&field.attributes) {
                let remaining_data = &data[offset..];
                let decoded_value = match &field.ty {
                    Type::Bytes => DecodedValue::Bytes(remaining_data.to_vec()),
                    Type::String => match String::from_utf8(remaining_data.to_vec()) {
                        Ok(s) => DecodedValue::String(s),
                        Err(_) => {
                            return Err(CoreError::codec(
                                "decode",
                                format!("remaining field '{}' contains invalid UTF-8", field.name),
                            ))
                        }
                    },
                    Type::Vec(inner) => {
                        // Vec<T> 剩余数据：按元素类型逐个解码
                        let mut vec_values = Vec::new();
                        let mut vec_offset = offset;
                        while vec_offset < data.len() {
                            let (val, consumed) = decode_value_with_endian(
                                data,
                                vec_offset,
                                inner,
                                self.schema,
                                endian,
                            )?;
                            vec_values.push(val);
                            vec_offset += consumed;
                        }
                        DecodedValue::Vec(vec_values)
                    }
                    _ => {
                        return Err(CoreError::codec(
                            "decode",
                            format!(
                                "remaining field '{}' must be Bytes, String, or Vec<T>",
                                field.name
                            ),
                        ));
                    }
                };
                fields.push((field.name.clone(), decoded_value));
                offset = data.len(); // 消费所有剩余数据
                continue;
            }

            // 检查是否为条件字段
            if let Some(condition) = get_if_attribute(&field.attributes) {
                // 查找条件字段的值
                let condition_value = fields
                    .iter()
                    .find(|(name, _)| name == condition)
                    .map(|(_, value)| value);

                match condition_value {
                    Some(DecodedValue::Bool(true)) => {
                        // 条件满足，正常解码
                        let (value, consumed) =
                            decode_value_with_endian(data, offset, &field.ty, self.schema, endian)?;
                        fields.push((field.name.clone(), value));
                        offset += consumed;
                    }
                    Some(DecodedValue::Bool(false)) => {
                        // 条件不满足，跳过此字段
                        continue;
                    }
                    Some(_) => {
                        return Err(CoreError::codec(
                            "decode",
                            format!("if condition field '{condition}' must be bool"),
                        ));
                    }
                    None => {
                        return Err(CoreError::codec(
                            "decode",
                            format!(
                                "if condition field '{condition}' not found or not yet decoded"
                            ),
                        ));
                    }
                }
            } else {
                // 普通字段解码
                let (value, consumed) =
                    decode_value_with_endian(data, offset, &field.ty, self.schema, endian)?;
                fields.push((field.name.clone(), value));
                offset += consumed;
            }
        }

        // 检查是否有多余的数据（如果存在 remaining 字段则允许）
        let has_remaining = self
            .struct_def
            .fields
            .iter()
            .any(|f| has_remaining_attribute(&f.attributes));
        if !has_remaining && offset < data.len() {
            return Err(CoreError::codec(
                "decode",
                format!(
                    "{} bytes of trailing data after decoding",
                    data.len() - offset
                ),
            ));
        }

        Ok(DecodedStruct {
            name: self.struct_def.name.clone(),
            fields,
        })
    }

    /// 获取结构体定义
    #[must_use]
    pub fn struct_def(&self) -> &StructDef {
        self.struct_def
    }

    #[allow(dead_code)]
    /// 计算编码后的大小
    fn calculate_encoded_size(&self) -> Result<usize> {
        let mut size = 0;
        for field in &self.struct_def.fields {
            // 跳过特殊字段，它们的大小需要动态计算或自动填充
            if has_auto_attribute(&field.attributes)
                || has_remaining_attribute(&field.attributes)
                || get_len_ref_attribute(&field.attributes).is_some()
                || get_checksum_attribute(&field.attributes).is_some()
            {
                // 为这些字段预留类型大小
                if let Ok(type_size) = calculate_type_size(&field.ty, self.schema) {
                    size += type_size;
                }
                continue;
            }
            match &field.value {
                Some(value) => size += calculate_value_size(value, &field.ty, self.schema)?,
                None => {
                    return Err(CoreError::codec(
                        "encode",
                        format!("field '{}' has no value", field.name),
                    ));
                }
            }
        }
        Ok(size)
    }

    /// 获取字段的字节序（支持字段级覆盖）
    fn get_field_endian(&self, field: &crate::ast::Field) -> Endianness {
        field
            .attributes
            .iter()
            .find_map(|attr| match attr {
                FieldAttribute::Endian(endian) => Some(*endian),
                _ => None,
            })
            .unwrap_or(self.default_endian)
    }

    /// 获取结构体的字节序
    fn get_struct_endian(&self) -> Endianness {
        self.struct_def.endian.unwrap_or(self.default_endian)
    }
}

/// Auto 字段信息，用于第二阶段回填
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AutoFieldInfo {
    /// 字段在 buffer 中的绝对偏移量
    offset: usize,
    /// 字段定义
    field: crate::ast::Field,
    /// 字段所在的结构体定义
    struct_def: StructDef,
}

/// 编码结构体并跟踪所有 auto 字段的位置
///
/// 返回：(编码后的字节, 字段偏移量映射, auto 字段信息列表)
fn encode_struct_with_auto_tracking(
    struct_def: &StructDef,
    field_values: &std::collections::HashMap<String, Value>,
    endian: Endianness,
    schema: &Schema,
    base_offset: usize,
) -> Result<(
    Vec<u8>,
    FieldOffsets,
    Vec<(usize, crate::ast::Field, StructDef)>,
)> {
    let mut buffer = Vec::new();
    let mut field_offsets: FieldOffsets = FieldOffsets::new();
    let mut auto_fields: Vec<(usize, crate::ast::Field, StructDef)> = Vec::new();
    let mut checksum_field_idx: Option<usize> = None;
    let checksum_start: usize = 0;

    // 辅助函数：获取字段字节序
    let get_field_endian = |field: &crate::ast::Field| -> Endianness {
        field
            .attributes
            .iter()
            .find_map(|attr| match attr {
                FieldAttribute::Endian(e) => Some(*e),
                _ => None,
            })
            .unwrap_or(endian)
    };

    // 预扫描位域，按位位置范围分组
    let mut bitfield_groups: std::collections::HashMap<usize, Vec<&crate::ast::Field>> =
        std::collections::HashMap::new();
    for field in &struct_def.fields {
        if let Some((_start, end)) = get_bits_attribute(&field.attributes) {
            let max_bit = end;
            let storage_key = if max_bit <= 7 {
                8
            } else if max_bit <= 15 {
                16
            } else if max_bit <= 31 {
                32
            } else {
                64
            };
            bitfield_groups.entry(storage_key).or_default().push(field);
        } else if let Some(pos) = get_bit_attribute(&field.attributes) {
            let storage_key = if pos <= 7 {
                8
            } else if pos <= 15 {
                16
            } else if pos <= 31 {
                32
            } else {
                64
            };
            bitfield_groups.entry(storage_key).or_default().push(field);
        }
    }
    let mut processed_bitfield_groups: std::collections::HashSet<usize> =
        std::collections::HashSet::new();

    // 第一遍：编码非特殊字段，记录位置和 auto 字段
    for (idx, field) in struct_def.fields.iter().enumerate() {
        let field_endian = get_field_endian(field);

        // 检查是否为 checksum 字段
        if get_checksum_attribute(&field.attributes).is_some() {
            checksum_field_idx = Some(idx);
            let size = calculate_type_size(&field.ty, schema)?;
            field_offsets.insert(field.name.clone(), (buffer.len(), size));
            buffer.resize(buffer.len() + size, 0);
            continue;
        }

        // 检查是否为 auto 字段 - 记录位置，稍后回填
        if has_auto_attribute(&field.attributes) {
            let size = calculate_type_size(&field.ty, schema)?;
            let absolute_offset = base_offset + buffer.len();
            field_offsets.insert(field.name.clone(), (buffer.len(), size));
            // 记录 auto 字段信息，用于第二阶段回填
            auto_fields.push((absolute_offset, field.clone(), struct_def.clone()));
            buffer.resize(buffer.len() + size, 0);
            continue;
        }

        // 检查是否为 len_ref 字段
        if get_len_ref_attribute(&field.attributes).is_some() {
            let size = calculate_type_size(&field.ty, schema)?;
            field_offsets.insert(field.name.clone(), (buffer.len(), size));
            buffer.resize(buffer.len() + size, 0);
            continue;
        }

        // 检查是否为 remaining 字段
        if has_remaining_attribute(&field.attributes) {
            field_offsets.insert(field.name.clone(), (buffer.len(), 0));
            continue;
        }

        // 检查是否为位域字段
        if let Some((_start_bit, end_bit)) = get_bits_attribute(&field.attributes) {
            let max_bit = end_bit;
            let storage_key = if max_bit <= 7 {
                8
            } else if max_bit <= 15 {
                16
            } else if max_bit <= 31 {
                32
            } else {
                64
            };
            if !processed_bitfield_groups.contains(&storage_key) {
                // 处理整个位域组
                if let Some(group_fields) = bitfield_groups.get(&storage_key) {
                    let group_endian = get_field_endian(field);
                    let mut combined_value: u64 = 0;
                    for gf in group_fields {
                        if let Some((s, e)) = get_bits_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, s, e);
                            }
                        } else if let Some(pos) = get_bit_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Bool(val)) = field_val {
                                let bit_val = u64::from(*val);
                                combined_value = set_bits(combined_value, bit_val, pos, pos);
                            } else if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, pos, pos);
                            }
                        }
                    }

                    let start = buffer.len();
                    match storage_key {
                        8 => buffer.push(combined_value as u8),
                        16 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u16).to_be_bytes(),
                                Endianness::Little => (combined_value as u16).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        32 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u32).to_be_bytes(),
                                Endianness::Little => (combined_value as u32).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        64 => {
                            let bytes = match group_endian {
                                Endianness::Big => combined_value.to_be_bytes(),
                                Endianness::Little => combined_value.to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        _ => unreachable!(),
                    }

                    let total_size = buffer.len() - start;
                    for gf in group_fields {
                        field_offsets.insert(gf.name.clone(), (start, total_size));
                    }
                }
                processed_bitfield_groups.insert(storage_key);
            }
            continue;
        } else if let Some(pos) = get_bit_attribute(&field.attributes) {
            let storage_key = if pos <= 7 {
                8
            } else if pos <= 15 {
                16
            } else if pos <= 31 {
                32
            } else {
                64
            };
            if !processed_bitfield_groups.contains(&storage_key) {
                // 处理整个位域组
                if let Some(group_fields) = bitfield_groups.get(&storage_key) {
                    let group_endian = get_field_endian(field);
                    let mut combined_value: u64 = 0;
                    for gf in group_fields {
                        if let Some((s, e)) = get_bits_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, s, e);
                            }
                        } else if let Some(p) = get_bit_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Bool(val)) = field_val {
                                let bit_val = u64::from(*val);
                                combined_value = set_bits(combined_value, bit_val, p, p);
                            } else if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, p, p);
                            }
                        }
                    }

                    let start = buffer.len();
                    match storage_key {
                        8 => buffer.push(combined_value as u8),
                        16 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u16).to_be_bytes(),
                                Endianness::Little => (combined_value as u16).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        32 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u32).to_be_bytes(),
                                Endianness::Little => (combined_value as u32).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        64 => {
                            let bytes = match group_endian {
                                Endianness::Big => combined_value.to_be_bytes(),
                                Endianness::Little => combined_value.to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        _ => unreachable!(),
                    }

                    let total_size = buffer.len() - start;
                    for gf in group_fields {
                        field_offsets.insert(gf.name.clone(), (start, total_size));
                    }
                }
                processed_bitfield_groups.insert(storage_key);
            }
            continue;
        }

        // 获取字段值
        let field_value = field_values
            .get(&field.name)
            .or(field.value.as_ref())
            .ok_or_else(|| {
                CoreError::codec("encode", format!("field '{}' has no value", field.name))
            })?;

        // 检查是否为条件字段
        if let Some(condition) = get_if_attribute(&field.attributes) {
            let condition_value = struct_def
                .fields
                .iter()
                .find(|f| f.name == condition)
                .and_then(|f| f.value.as_ref())
                .or_else(|| field_values.get(condition));

            match condition_value {
                Some(Value::Bool(true)) => {
                    let start = buffer.len();
                    let current_offset = base_offset + buffer.len();
                    encode_value_with_auto_tracking(
                        &mut buffer,
                        field_value,
                        &field.ty,
                        field_endian,
                        schema,
                        &mut auto_fields,
                        current_offset,
                    )?;
                    field_offsets.insert(field.name.clone(), (start, buffer.len() - start));
                }
                Some(Value::Bool(false)) => {
                    continue;
                }
                Some(_) => {
                    return Err(CoreError::codec(
                        "encode",
                        format!("if condition field '{condition}' must be bool"),
                    ));
                }
                None => {
                    return Err(CoreError::codec(
                        "encode",
                        format!("if condition field '{condition}' not found or has no value"),
                    ));
                }
            }
            continue;
        }

        // 普通字段直接编码
        let start = buffer.len();
        let current_offset = base_offset + buffer.len();
        encode_value_with_auto_tracking(
            &mut buffer,
            field_value,
            &field.ty,
            field_endian,
            schema,
            &mut auto_fields,
            current_offset,
        )?;
        field_offsets.insert(field.name.clone(), (start, buffer.len() - start));
    }

    // 计算 len_ref 字段
    for field in &struct_def.fields {
        if let Some(ref_field_name) = get_len_ref_attribute(&field.attributes) {
            if let Some(&(offset, _size)) = field_offsets.get(&field.name) {
                // 根据引用的字段计算长度
                let length = if let Some(ref_value) = field_values.get(ref_field_name as &str) {
                    match ref_value {
                        Value::Array(arr) => arr.len() as u64,
                        Value::Bytes(bytes) => bytes.len() as u64,
                        Value::String(s) => s.len() as u64,
                        _ => 0,
                    }
                } else if let Some(ref_field) =
                    struct_def.fields.iter().find(|f| f.name == ref_field_name)
                {
                    if let Some(ref_value) = &ref_field.value {
                        match ref_value {
                            Value::Array(arr) => arr.len() as u64,
                            Value::Bytes(bytes) => bytes.len() as u64,
                            Value::String(s) => s.len() as u64,
                            _ => 0,
                        }
                    } else {
                        0
                    }
                } else {
                    0
                };

                let field_endian = get_field_endian(field);
                match field.ty {
                    Type::U8 => buffer[offset] = length as u8,
                    Type::U16 => {
                        let bytes = match field_endian {
                            Endianness::Big => (length as u16).to_be_bytes(),
                            Endianness::Little => (length as u16).to_le_bytes(),
                        };
                        buffer[offset..offset + 2].copy_from_slice(&bytes);
                    }
                    Type::U32 => {
                        let bytes = match field_endian {
                            Endianness::Big => (length as u32).to_be_bytes(),
                            Endianness::Little => (length as u32).to_le_bytes(),
                        };
                        buffer[offset..offset + 4].copy_from_slice(&bytes);
                    }
                    Type::U64 => {
                        let bytes = match field_endian {
                            Endianness::Big => length.to_be_bytes(),
                            Endianness::Little => length.to_le_bytes(),
                        };
                        buffer[offset..offset + 8].copy_from_slice(&bytes);
                    }
                    _ => {}
                }
            }
        }
    }

    // 计算 remaining 字段
    for field in &struct_def.fields {
        if has_remaining_attribute(&field.attributes) {
            if let Some(&(_offset, _)) = field_offsets.get(&field.name) {
                // remaining 字段不写入任何内容，只是标记位置
                // 实际的数据在编码完成后由调用者处理
                let _remaining_data: Vec<u8> = Vec::new();
            }
        }
    }

    // 计算 checksum 字段
    if let Some(idx) = checksum_field_idx {
        let field = &struct_def.fields[idx];
        if let Some(&(offset, _)) = field_offsets.get(&field.name) {
            if let Some(algorithm) = get_checksum_attribute(&field.attributes) {
                let checksum_data = &buffer[checksum_start..offset];
                let checksum = calculate_checksum(checksum_data, algorithm);

                let field_endian = get_field_endian(field);
                match field.ty {
                    Type::U8 => buffer[offset] = checksum as u8,
                    Type::U16 => {
                        let bytes = match field_endian {
                            Endianness::Big => (checksum as u16).to_be_bytes(),
                            Endianness::Little => (checksum as u16).to_le_bytes(),
                        };
                        buffer[offset..offset + 2].copy_from_slice(&bytes);
                    }
                    Type::U32 => {
                        let bytes = match field_endian {
                            Endianness::Big => (checksum as u32).to_be_bytes(),
                            Endianness::Little => (checksum as u32).to_le_bytes(),
                        };
                        buffer[offset..offset + 4].copy_from_slice(&bytes);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok((buffer, field_offsets, auto_fields))
}

/// 编码值并跟踪 auto 字段（用于嵌套结构体）
fn encode_value_with_auto_tracking(
    buffer: &mut Vec<u8>,
    value: &Value,
    ty: &Type,
    endian: Endianness,
    schema: &Schema,
    auto_fields: &mut Vec<(usize, crate::ast::Field, StructDef)>,
    base_offset: usize,
) -> Result<()> {
    match (value, ty) {
        (Value::Struct(struct_name, fields), Type::Custom(type_name)) => {
            if struct_name != type_name {
                return Err(CoreError::codec(
                    "encode",
                    format!("struct name mismatch: expected {type_name}, got {struct_name}"),
                ));
            }
            let struct_def = schema.get_struct(type_name).ok_or_else(|| {
                CoreError::codec(
                    "encode",
                    format!("struct '{type_name}' not found in schema"),
                )
            })?;

            let nested_field_values: std::collections::HashMap<String, Value> =
                fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

            // 递归编码嵌套结构体，传递 auto_fields 收集器
            let (nested_buffer, _, nested_auto_fields) = encode_struct_with_auto_tracking(
                struct_def,
                &nested_field_values,
                endian,
                schema,
                base_offset,
            )?;

            // 将嵌套结构体的 auto 字段信息合并到主列表
            auto_fields.extend(nested_auto_fields);
            buffer.extend_from_slice(&nested_buffer);
        }
        _ => {
            // 其他类型使用原有的编码逻辑
            encode_value_with_endian(buffer, value, ty, endian, schema)?;
        }
    }
    Ok(())
}

/// 检查是否有 auto 属性
fn has_auto_attribute(attrs: &[FieldAttribute]) -> bool {
    attrs
        .iter()
        .any(|attr| matches!(attr, FieldAttribute::Auto(_)))
}

/// 检查是否有 remaining 属性
fn has_remaining_attribute(attrs: &[FieldAttribute]) -> bool {
    attrs
        .iter()
        .any(|attr| matches!(attr, FieldAttribute::Remaining))
}

/// 检查是否有 `len_ref` 属性
fn get_len_ref_attribute(attrs: &[FieldAttribute]) -> Option<&str> {
    attrs.iter().find_map(|attr| match attr {
        FieldAttribute::LenRef(field_name) => Some(field_name.as_str()),
        _ => None,
    })
}

/// 获取 checksum 算法
fn get_checksum_attribute(attrs: &[FieldAttribute]) -> Option<crate::ast::ChecksumAlgorithm> {
    attrs.iter().find_map(|attr| match attr {
        FieldAttribute::Checksum(algo) => Some(*algo),
        _ => None,
    })
}

/// 获取 bits 属性 (start, end)
fn get_bits_attribute(attrs: &[FieldAttribute]) -> Option<(usize, usize)> {
    attrs.iter().find_map(|attr| match attr {
        FieldAttribute::Bits(start, end) => Some((*start, *end)),
        _ => None,
    })
}

/// 获取 bit 属性 (position)
fn get_bit_attribute(attrs: &[FieldAttribute]) -> Option<usize> {
    attrs.iter().find_map(|attr| match attr {
        FieldAttribute::Bit(pos) => Some(*pos),
        _ => None,
    })
}

/// 获取 if 条件属性
fn get_if_attribute(attrs: &[FieldAttribute]) -> Option<&str> {
    attrs.iter().find_map(|attr| match attr {
        FieldAttribute::If(cond) => Some(cond.as_str()),
        _ => None,
    })
}

/// 提取位域值
fn extract_bits(value: u64, start: usize, end: usize) -> u64 {
    let mask = ((1u64 << (end - start + 1)) - 1) << start;
    (value & mask) >> start
}

/// 设置位域值
fn set_bits(original: u64, new_value: u64, start: usize, end: usize) -> u64 {
    let mask = ((1u64 << (end - start + 1)) - 1) << start;
    (original & !mask) | ((new_value << start) & mask)
}

/// 计算校验和
fn calculate_checksum(data: &[u8], algorithm: crate::ast::ChecksumAlgorithm) -> u64 {
    use crate::ast::ChecksumAlgorithm;
    match algorithm {
        ChecksumAlgorithm::Crc8 => {
            // 简单 CRC-8 实现
            let mut crc: u8 = 0;
            for &byte in data {
                crc ^= byte;
                for _ in 0..8 {
                    if crc & 0x80 != 0 {
                        crc = (crc << 1) ^ 0x07;
                    } else {
                        crc <<= 1;
                    }
                }
            }
            u64::from(crc)
        }
        ChecksumAlgorithm::Crc16 => {
            // Modbus CRC-16
            let mut crc: u16 = 0xFFFF;
            for &byte in data {
                crc ^= u16::from(byte);
                for _ in 0..8 {
                    if crc & 0x0001 != 0 {
                        crc = (crc >> 1) ^ 0xA001;
                    } else {
                        crc >>= 1;
                    }
                }
            }
            u64::from(crc)
        }
        ChecksumAlgorithm::Crc32 => {
            // IEEE 802.3 CRC-32
            let mut crc: u32 = 0xFFFFFFFF;
            for &byte in data {
                crc ^= u32::from(byte);
                for _ in 0..8 {
                    if crc & 1 != 0 {
                        crc = (crc >> 1) ^ 0xEDB88320;
                    } else {
                        crc >>= 1;
                    }
                }
            }
            u64::from(!crc)
        }
        ChecksumAlgorithm::Xor => {
            // XOR 校验
            u64::from(data.iter().fold(0u8, |acc, &b| acc ^ b))
        }
        ChecksumAlgorithm::Sum => {
            // 字节累加和
            data.iter()
                .fold(0u64, |acc, &b| acc.wrapping_add(u64::from(b)))
        }
    }
}

/// 编码值到缓冲区（带字节序）
fn encode_value_with_endian(
    buffer: &mut Vec<u8>,
    value: &Value,
    ty: &Type,
    endian: Endianness,
    schema: &Schema,
) -> Result<()> {
    match (value, ty) {
        (Value::Integer(n), Type::U8) => buffer.push(*n as u8),
        (Value::Integer(n), Type::U16) => {
            let bytes = match endian {
                Endianness::Big => (*n as u16).to_be_bytes(),
                Endianness::Little => (*n as u16).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Integer(n), Type::U32) => {
            let bytes = match endian {
                Endianness::Big => (*n as u32).to_be_bytes(),
                Endianness::Little => (*n as u32).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Integer(n), Type::U64) => {
            let bytes = match endian {
                Endianness::Big => (*n as u64).to_be_bytes(),
                Endianness::Little => (*n as u64).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Integer(n), Type::U128) => {
            let bytes = match endian {
                Endianness::Big => (*n as u128).to_be_bytes(),
                Endianness::Little => (*n as u128).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Integer(n), Type::I8) => buffer.push(*n as i8 as u8),
        (Value::Integer(n), Type::I16) => {
            let bytes = match endian {
                Endianness::Big => (*n as i16).to_be_bytes(),
                Endianness::Little => (*n as i16).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Integer(n), Type::I32) => {
            let bytes = match endian {
                Endianness::Big => (*n as i32).to_be_bytes(),
                Endianness::Little => (*n as i32).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Integer(n), Type::I64) => {
            let bytes = match endian {
                Endianness::Big => (*n as i64).to_be_bytes(),
                Endianness::Little => (*n as i64).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Integer(n), Type::I128) => {
            let bytes = match endian {
                Endianness::Big => n.to_be_bytes(),
                Endianness::Little => n.to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Float(f), Type::F32) => {
            let bytes = match endian {
                Endianness::Big => (*f as f32).to_be_bytes(),
                Endianness::Little => (*f as f32).to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Float(f), Type::F64) => {
            let bytes = match endian {
                Endianness::Big => f.to_be_bytes(),
                Endianness::Little => f.to_le_bytes(),
            };
            buffer.extend_from_slice(&bytes);
        }
        (Value::Bool(b), Type::Bool) => buffer.push(u8::from(*b)),
        (Value::String(s), Type::String) => {
            // 字符串编码: 4字节长度(大端序) + 内容
            let bytes = s.as_bytes();
            if schema.is_prefix_enabled() {
                buffer.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            }
            buffer.extend_from_slice(bytes);
        }
        (Value::Bytes(b), Type::Bytes) => {
            // 字节数组编码: 4字节长度(大端序) + 内容
            if schema.is_prefix_enabled() {
                buffer.extend_from_slice(&(b.len() as u32).to_be_bytes());
            }
            buffer.extend_from_slice(b);
        }
        // 十六进制整数作为 Bytes
        (Value::Integer(n), Type::Bytes) => {
            let (bytes, len) = if *n <= i128::from(u8::MAX) {
                ((*n as u8).to_be_bytes().to_vec(), 1)
            } else if *n <= i128::from(u16::MAX) {
                let b = match endian {
                    Endianness::Big => (*n as u16).to_be_bytes().to_vec(),
                    Endianness::Little => (*n as u16).to_le_bytes().to_vec(),
                };
                (b, 2)
            } else if *n <= i128::from(u32::MAX) {
                let b = match endian {
                    Endianness::Big => (*n as u32).to_be_bytes().to_vec(),
                    Endianness::Little => (*n as u32).to_le_bytes().to_vec(),
                };
                (b, 4)
            } else if *n <= i128::from(u64::MAX) {
                let b = match endian {
                    Endianness::Big => (*n as u64).to_be_bytes().to_vec(),
                    Endianness::Little => (*n as u64).to_le_bytes().to_vec(),
                };
                (b, 8)
            } else {
                let b = match endian {
                    Endianness::Big => (*n as u128).to_be_bytes().to_vec(),
                    Endianness::Little => (*n as u128).to_le_bytes().to_vec(),
                };
                (b, 16)
            };
            if schema.is_prefix_enabled() {
                buffer.extend_from_slice(&(len as u32).to_be_bytes());
            }
            buffer.extend_from_slice(&bytes);
        }
        (Value::Array(arr), Type::Vec(inner_ty)) => {
            // Vec编码: 4字节长度(大端序) + 元素
            if schema.is_prefix_enabled() {
                buffer.extend_from_slice(&(arr.len() as u32).to_be_bytes());
            }
            for item in arr {
                encode_value_with_endian(buffer, item, inner_ty, endian, schema)?;
            }
        }
        (Value::Array(arr), Type::Array(inner_ty, expected_size)) => {
            if arr.len() != *expected_size {
                return Err(CoreError::codec(
                    "encode",
                    format!(
                        "array size mismatch: expected {}, got {}",
                        expected_size,
                        arr.len()
                    ),
                ));
            }
            for item in arr {
                encode_value_with_endian(buffer, item, inner_ty, endian, schema)?;
            }
        }
        (Value::Struct(struct_name, fields), Type::Custom(type_name)) => {
            if struct_name != type_name {
                return Err(CoreError::codec(
                    "encode",
                    format!("struct name mismatch: expected {type_name}, got {struct_name}"),
                ));
            }
            let struct_def = schema.get_struct(type_name).ok_or_else(|| {
                CoreError::codec(
                    "encode",
                    format!("struct '{type_name}' not found in schema"),
                )
            })?;

            // 将 Vec<(String, Value)> 转换为 HashMap<String, Value>
            let field_values: std::collections::HashMap<String, Value> =
                fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

            // 结构体级别字节序已从 StructDef 移除，使用传入的字节序
            let struct_endian = endian;

            let (encoded, _) =
                encode_struct_by_definition(struct_def, &field_values, struct_endian, schema)?;
            buffer.extend_from_slice(&encoded);
        }
        _ => {
            return Err(CoreError::codec(
                "encode",
                format!("value {value:?} does not match type {ty:?}"),
            ));
        }
    }
    Ok(())
}

/// 根据结构体定义编码字段（处理所有属性：auto、checksum、len_ref、remaining、位域、if）
///
/// 这是 `Codec::encode` 的核心逻辑提取，用于支持嵌套结构体中的属性处理
fn encode_struct_by_definition(
    struct_def: &StructDef,
    field_values: &std::collections::HashMap<String, Value>,
    endian: Endianness,
    schema: &Schema,
) -> Result<(Vec<u8>, FieldOffsets)> {
    let mut buffer = Vec::new();
    let mut field_offsets: FieldOffsets = FieldOffsets::new();
    let mut checksum_field_idx: Option<usize> = None;
    let checksum_start: usize = 0;

    // 辅助函数：获取字段字节序
    let get_field_endian = |field: &crate::ast::Field| -> Endianness {
        field
            .attributes
            .iter()
            .find_map(|attr| match attr {
                FieldAttribute::Endian(e) => Some(*e),
                _ => None,
            })
            .unwrap_or(endian)
    };

    // 预扫描位域，按位位置范围分组
    let mut bitfield_groups: std::collections::HashMap<usize, Vec<&crate::ast::Field>> =
        std::collections::HashMap::new();
    for field in &struct_def.fields {
        if let Some((_start, end)) = get_bits_attribute(&field.attributes) {
            let max_bit = end;
            let storage_key = if max_bit <= 7 {
                8
            } else if max_bit <= 15 {
                16
            } else if max_bit <= 31 {
                32
            } else {
                64
            };
            bitfield_groups.entry(storage_key).or_default().push(field);
        } else if let Some(pos) = get_bit_attribute(&field.attributes) {
            let storage_key = if pos <= 7 {
                8
            } else if pos <= 15 {
                16
            } else if pos <= 31 {
                32
            } else {
                64
            };
            bitfield_groups.entry(storage_key).or_default().push(field);
        }
    }
    let mut processed_bitfield_groups: std::collections::HashSet<usize> =
        std::collections::HashSet::new();

    // 第一遍：编码非特殊字段，记录位置
    for (idx, field) in struct_def.fields.iter().enumerate() {
        let field_endian = get_field_endian(field);

        // 检查是否为 checksum 字段
        if get_checksum_attribute(&field.attributes).is_some() {
            checksum_field_idx = Some(idx);
            let size = calculate_type_size(&field.ty, schema)?;
            field_offsets.insert(field.name.clone(), (buffer.len(), size));
            buffer.resize(buffer.len() + size, 0);
            continue;
        }

        // 检查是否为 auto 字段
        if has_auto_attribute(&field.attributes) {
            let size = calculate_type_size(&field.ty, schema)?;
            field_offsets.insert(field.name.clone(), (buffer.len(), size));
            buffer.resize(buffer.len() + size, 0);
            continue;
        }

        // 检查是否为 len_ref 字段
        if get_len_ref_attribute(&field.attributes).is_some() {
            let size = calculate_type_size(&field.ty, schema)?;
            field_offsets.insert(field.name.clone(), (buffer.len(), size));
            buffer.resize(buffer.len() + size, 0);
            continue;
        }

        // 检查是否为 remaining 字段
        if has_remaining_attribute(&field.attributes) {
            field_offsets.insert(field.name.clone(), (buffer.len(), 0));
            continue;
        }

        // 检查是否为位域字段
        if let Some((_start_bit, end_bit)) = get_bits_attribute(&field.attributes) {
            let max_bit = end_bit;
            let storage_key = if max_bit <= 7 {
                8
            } else if max_bit <= 15 {
                16
            } else if max_bit <= 31 {
                32
            } else {
                64
            };
            if !processed_bitfield_groups.contains(&storage_key) {
                // 处理整个位域组
                if let Some(group_fields) = bitfield_groups.get(&storage_key) {
                    let group_endian = get_field_endian(field);
                    let mut combined_value: u64 = 0;
                    for gf in group_fields {
                        if let Some((s, e)) = get_bits_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, s, e);
                            }
                        } else if let Some(p) = get_bit_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Bool(val)) = field_val {
                                let bit_val = u64::from(*val);
                                combined_value = set_bits(combined_value, bit_val, p, p);
                            } else if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, p, p);
                            }
                        }
                    }

                    let start = buffer.len();
                    match storage_key {
                        8 => buffer.push(combined_value as u8),
                        16 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u16).to_be_bytes(),
                                Endianness::Little => (combined_value as u16).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        32 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u32).to_be_bytes(),
                                Endianness::Little => (combined_value as u32).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        64 => {
                            let bytes = match group_endian {
                                Endianness::Big => combined_value.to_be_bytes(),
                                Endianness::Little => combined_value.to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        _ => unreachable!(),
                    }

                    let total_size = buffer.len() - start;
                    for gf in group_fields {
                        field_offsets.insert(gf.name.clone(), (start, total_size));
                    }
                }
                processed_bitfield_groups.insert(storage_key);
            }
            continue;
        } else if let Some(pos) = get_bit_attribute(&field.attributes) {
            let storage_key = if pos <= 7 {
                8
            } else if pos <= 15 {
                16
            } else if pos <= 31 {
                32
            } else {
                64
            };
            if !processed_bitfield_groups.contains(&storage_key) {
                // 处理整个位域组
                if let Some(group_fields) = bitfield_groups.get(&storage_key) {
                    let group_endian = get_field_endian(field);
                    let mut combined_value: u64 = 0;
                    for gf in group_fields {
                        if let Some((s, e)) = get_bits_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, s, e);
                            }
                        } else if let Some(p) = get_bit_attribute(&gf.attributes) {
                            let field_val = field_values.get(&gf.name).or(gf.value.as_ref());
                            if let Some(Value::Bool(val)) = field_val {
                                let bit_val = u64::from(*val);
                                combined_value = set_bits(combined_value, bit_val, p, p);
                            } else if let Some(Value::Integer(val)) = field_val {
                                combined_value = set_bits(combined_value, *val as u64, p, p);
                            }
                        }
                    }

                    let start = buffer.len();
                    match storage_key {
                        8 => buffer.push(combined_value as u8),
                        16 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u16).to_be_bytes(),
                                Endianness::Little => (combined_value as u16).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        32 => {
                            let bytes = match group_endian {
                                Endianness::Big => (combined_value as u32).to_be_bytes(),
                                Endianness::Little => (combined_value as u32).to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        64 => {
                            let bytes = match group_endian {
                                Endianness::Big => combined_value.to_be_bytes(),
                                Endianness::Little => combined_value.to_le_bytes(),
                            };
                            buffer.extend_from_slice(&bytes);
                        }
                        _ => unreachable!(),
                    }

                    let total_size = buffer.len() - start;
                    for gf in group_fields {
                        field_offsets.insert(gf.name.clone(), (start, total_size));
                    }
                }
                processed_bitfield_groups.insert(storage_key);
            }
            continue;
        }

        // 获取字段值（从 field_values 映射或 struct_def 默认值）
        let field_value = field_values
            .get(&field.name)
            .or(field.value.as_ref())
            .ok_or_else(|| {
                CoreError::codec("encode", format!("field '{}' has no value", field.name))
            })?;

        // 检查是否为条件字段
        if let Some(condition) = get_if_attribute(&field.attributes) {
            let condition_value = struct_def
                .fields
                .iter()
                .find(|f| f.name == condition)
                .and_then(|f| f.value.as_ref())
                .or_else(|| field_values.get(condition));

            match condition_value {
                Some(Value::Bool(true)) => {
                    let start = buffer.len();
                    encode_value_with_endian(
                        &mut buffer,
                        field_value,
                        &field.ty,
                        field_endian,
                        schema,
                    )?;
                    field_offsets.insert(field.name.clone(), (start, buffer.len() - start));
                }
                Some(Value::Bool(false)) => {
                    continue; // 条件不满足，跳过
                }
                Some(_) => {
                    return Err(CoreError::codec(
                        "encode",
                        format!("if condition field '{condition}' must be bool"),
                    ));
                }
                None => {
                    return Err(CoreError::codec(
                        "encode",
                        format!("if condition field '{condition}' not found or has no value"),
                    ));
                }
            }
            continue;
        }

        // 普通字段直接编码
        let start = buffer.len();
        encode_value_with_endian(&mut buffer, field_value, &field.ty, field_endian, schema)?;
        field_offsets.insert(field.name.clone(), (start, buffer.len() - start));
    }

    // 第二遍：计算 len_ref 字段（这些字段的值取决于其他字段的长度）
    for field in &struct_def.fields {
        if let Some(ref_field_name) = get_len_ref_attribute(&field.attributes) {
            if let Some(&(offset, _size)) = field_offsets.get(&field.name) {
                // 找到被引用的字段
                let ref_field = struct_def
                    .fields
                    .iter()
                    .find(|f| f.name == ref_field_name)
                    .ok_or_else(|| {
                        CoreError::codec(
                            "encode",
                            format!("len_ref references unknown field '{ref_field_name}'"),
                        )
                    })?;

                let ref_value = field_values
                    .get(&ref_field.name)
                    .or(ref_field.value.as_ref());

                // 计算被引用字段的长度（元素个数，不是字节数）
                let length = if has_remaining_attribute(&ref_field.attributes) {
                    // remaining 字段使用实际值的字节大小
                    if let Some(Value::Bytes(bytes)) = ref_value {
                        bytes.len() as u64
                    } else if let Some(Value::String(s)) = ref_value {
                        s.len() as u64
                    } else {
                        0
                    }
                } else {
                    // 对于 Vec/Bytes/String，使用元素个数
                    match &ref_field.ty {
                        Type::Vec(_) => {
                            if let Some(Value::Array(arr)) = ref_value {
                                arr.len() as u64
                            } else {
                                0
                            }
                        }
                        Type::Bytes => {
                            if let Some(Value::Bytes(bytes)) = ref_value {
                                bytes.len() as u64
                            } else {
                                0
                            }
                        }
                        Type::String => {
                            if let Some(Value::String(s)) = ref_value {
                                s.len() as u64
                            } else {
                                0
                            }
                        }
                        // 对于固定大小类型，返回1（表示一个元素）
                        _ => 1,
                    }
                };

                // 根据字段类型写入长度值
                let field_endian = get_field_endian(field);
                match field.ty {
                    Type::U8 => buffer[offset] = length as u8,
                    Type::U16 => {
                        let bytes = match field_endian {
                            Endianness::Big => (length as u16).to_be_bytes(),
                            Endianness::Little => (length as u16).to_le_bytes(),
                        };
                        buffer[offset..offset + 2].copy_from_slice(&bytes);
                    }
                    Type::U32 => {
                        let bytes = match field_endian {
                            Endianness::Big => (length as u32).to_be_bytes(),
                            Endianness::Little => (length as u32).to_le_bytes(),
                        };
                        buffer[offset..offset + 4].copy_from_slice(&bytes);
                    }
                    Type::U64 => {
                        let bytes = match field_endian {
                            Endianness::Big => length.to_be_bytes(),
                            Endianness::Little => length.to_le_bytes(),
                        };
                        buffer[offset..offset + 8].copy_from_slice(&bytes);
                    }
                    _ => {
                        return Err(CoreError::codec(
                            "encode",
                            format!(
                                "len_ref field '{}' must be an unsigned integer type",
                                field.name
                            ),
                        ));
                    }
                }
            }
        }
    }

    // 编码 remaining 字段
    for field in &struct_def.fields {
        if has_remaining_attribute(&field.attributes) {
            if let Some(&(offset, _)) = field_offsets.get(&field.name) {
                // remaining 字段应该包含所有剩余数据
                let remaining_value = field_values.get(&field.name).or(field.value.as_ref());
                if let Some(Value::Bytes(bytes)) = remaining_value {
                    // 验证这是最后一个字段
                    if offset != buffer.len() {
                        return Err(CoreError::codec(
                            "encode",
                            format!("remaining field '{}' must be the last field", field.name),
                        ));
                    }
                    buffer.extend_from_slice(bytes);
                } else {
                    return Err(CoreError::codec(
                        "encode",
                        format!("remaining field '{}' must be of Bytes type", field.name),
                    ));
                }
            }
        }
    }

    // 计算并写入 checksum
    if let Some(idx) = checksum_field_idx {
        let field = &struct_def.fields[idx];
        if let Some(algorithm) = get_checksum_attribute(&field.attributes) {
            // 计算从开始到 checksum 字段前的校验和
            let checksum = calculate_checksum(&buffer[checksum_start..], algorithm);

            // 找到 checksum 字段的位置
            if let Some(&(offset, _)) = field_offsets.get(&field.name) {
                // 根据字段类型写入校验和
                let field_endian = get_field_endian(field);
                match field.ty {
                    Type::U8 => buffer[offset] = checksum as u8,
                    Type::U16 => {
                        let bytes = match field_endian {
                            Endianness::Big => (checksum as u16).to_be_bytes(),
                            Endianness::Little => (checksum as u16).to_le_bytes(),
                        };
                        buffer[offset..offset + 2].copy_from_slice(&bytes);
                    }
                    Type::U32 => {
                        let bytes = match field_endian {
                            Endianness::Big => (checksum as u32).to_be_bytes(),
                            Endianness::Little => (checksum as u32).to_le_bytes(),
                        };
                        buffer[offset..offset + 4].copy_from_slice(&bytes);
                    }
                    Type::U64 => {
                        let bytes = match field_endian {
                            Endianness::Big => checksum.to_be_bytes(),
                            Endianness::Little => checksum.to_le_bytes(),
                        };
                        buffer[offset..offset + 8].copy_from_slice(&bytes);
                    }
                    _ => {
                        return Err(CoreError::codec(
                            "encode",
                            format!(
                                "checksum field '{}' must be an unsigned integer type",
                                field.name
                            ),
                        ));
                    }
                }
            }
        }
    }

    Ok((buffer, field_offsets))
}

/// 编码值到缓冲区（默认大端序）
#[allow(dead_code)]
fn encode_value(buffer: &mut Vec<u8>, value: &Value, ty: &Type, schema: &Schema) -> Result<()> {
    encode_value_with_endian(buffer, value, ty, Endianness::Big, schema)
}

/// 从缓冲区解码值（默认大端序）
#[allow(dead_code)]
fn decode_value(
    data: &[u8],
    offset: usize,
    ty: &Type,
    schema: &Schema,
) -> Result<(DecodedValue, usize)> {
    decode_value_with_endian(data, offset, ty, schema, Endianness::Big)
}

/// 从缓冲区解码值（带字节序）
fn decode_value_with_endian(
    data: &[u8],
    offset: usize,
    ty: &Type,
    schema: &Schema,
    endian: Endianness,
) -> Result<(DecodedValue, usize)> {
    if offset >= data.len() {
        return Err(CoreError::codec(
            "decode",
            format!("unexpected end of data at offset {offset}"),
        ));
    }

    match ty {
        Type::U8 => {
            check_buffer_size(data, offset, 1)?;
            Ok((DecodedValue::U8(data[offset]), 1))
        }
        Type::U16 => {
            check_buffer_size(data, offset, 2)?;
            let bytes = [data[offset], data[offset + 1]];
            let value = match endian {
                Endianness::Big => u16::from_be_bytes(bytes),
                Endianness::Little => u16::from_le_bytes(bytes),
            };
            Ok((DecodedValue::U16(value), 2))
        }
        Type::U32 => {
            check_buffer_size(data, offset, 4)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            let value = match endian {
                Endianness::Big => u32::from_be_bytes(bytes),
                Endianness::Little => u32::from_le_bytes(bytes),
            };
            Ok((DecodedValue::U32(value), 4))
        }
        Type::U64 => {
            check_buffer_size(data, offset, 8)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ];
            let value = match endian {
                Endianness::Big => u64::from_be_bytes(bytes),
                Endianness::Little => u64::from_le_bytes(bytes),
            };
            Ok((DecodedValue::U64(value), 8))
        }
        Type::U128 => {
            check_buffer_size(data, offset, 16)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ];
            let value = match endian {
                Endianness::Big => u128::from_be_bytes(bytes),
                Endianness::Little => u128::from_le_bytes(bytes),
            };
            Ok((DecodedValue::U128(value), 16))
        }
        Type::I8 => {
            check_buffer_size(data, offset, 1)?;
            Ok((DecodedValue::I8(data[offset] as i8), 1))
        }
        Type::I16 => {
            check_buffer_size(data, offset, 2)?;
            let bytes = [data[offset], data[offset + 1]];
            let value = match endian {
                Endianness::Big => i16::from_be_bytes(bytes),
                Endianness::Little => i16::from_le_bytes(bytes),
            };
            Ok((DecodedValue::I16(value), 2))
        }
        Type::I32 => {
            check_buffer_size(data, offset, 4)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            let value = match endian {
                Endianness::Big => i32::from_be_bytes(bytes),
                Endianness::Little => i32::from_le_bytes(bytes),
            };
            Ok((DecodedValue::I32(value), 4))
        }
        Type::I64 => {
            check_buffer_size(data, offset, 8)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ];
            let value = match endian {
                Endianness::Big => i64::from_be_bytes(bytes),
                Endianness::Little => i64::from_le_bytes(bytes),
            };
            Ok((DecodedValue::I64(value), 8))
        }
        Type::I128 => {
            check_buffer_size(data, offset, 16)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ];
            let value = match endian {
                Endianness::Big => i128::from_be_bytes(bytes),
                Endianness::Little => i128::from_le_bytes(bytes),
            };
            Ok((DecodedValue::I128(value), 16))
        }
        Type::F32 => {
            check_buffer_size(data, offset, 4)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            let value = match endian {
                Endianness::Big => f32::from_be_bytes(bytes),
                Endianness::Little => f32::from_le_bytes(bytes),
            };
            Ok((DecodedValue::F32(value), 4))
        }
        Type::F64 => {
            check_buffer_size(data, offset, 8)?;
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ];
            let value = match endian {
                Endianness::Big => f64::from_be_bytes(bytes),
                Endianness::Little => f64::from_le_bytes(bytes),
            };
            Ok((DecodedValue::F64(value), 8))
        }
        Type::Bool => {
            check_buffer_size(data, offset, 1)?;
            Ok((DecodedValue::Bool(data[offset] != 0), 1))
        }
        Type::String => {
            if !schema.is_prefix_enabled() {
                return Err(CoreError::codec(
                    "decode",
                    "prefix disabled: String must use #[len_ref] or #[remaining]".to_string(),
                ));
            }
            // 字符串解码: 4字节长度(大端序) + 内容
            check_buffer_size(data, offset, 4)?;
            let len_bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            let len = u32::from_be_bytes(len_bytes) as usize;
            check_buffer_size(data, offset + 4, len)?;
            let string_data = &data[offset + 4..offset + 4 + len];
            let value = String::from_utf8(string_data.to_vec())
                .map_err(|_| CoreError::codec("decode", "invalid UTF-8 string".to_string()))?;
            Ok((DecodedValue::String(value), 4 + len))
        }
        Type::Bytes => {
            if !schema.is_prefix_enabled() {
                return Err(CoreError::codec(
                    "decode",
                    "prefix disabled: Bytes must use #[len_ref] or #[remaining]".to_string(),
                ));
            }
            // 字节数组解码: 4字节长度(大端序) + 内容
            check_buffer_size(data, offset, 4)?;
            let len_bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            let len = u32::from_be_bytes(len_bytes) as usize;
            check_buffer_size(data, offset + 4, len)?;
            let bytes_data = data[offset + 4..offset + 4 + len].to_vec();
            Ok((DecodedValue::Bytes(bytes_data), 4 + len))
        }
        Type::Vec(inner_ty) => {
            if !schema.is_prefix_enabled() {
                return Err(CoreError::codec(
                    "decode",
                    "prefix disabled: Vec must use #[len_ref] or #[remaining]".to_string(),
                ));
            }
            // Vec解码: 4字节长度(大端序) + 元素
            check_buffer_size(data, offset, 4)?;
            let len_bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut values = Vec::with_capacity(len);
            let mut current_offset = offset + 4;
            for _ in 0..len {
                let (value, consumed) =
                    decode_value_with_endian(data, current_offset, inner_ty, schema, endian)?;
                values.push(value);
                current_offset += consumed;
            }
            Ok((DecodedValue::Vec(values), current_offset - offset))
        }
        Type::Array(inner_ty, size) => {
            let mut values = Vec::with_capacity(*size);
            let mut current_offset = offset;
            for _ in 0..*size {
                let (value, consumed) =
                    decode_value_with_endian(data, current_offset, inner_ty, schema, endian)?;
                values.push(value);
                current_offset += consumed;
            }
            Ok((DecodedValue::Vec(values), current_offset - offset))
        }
        Type::Custom(struct_name) => {
            let struct_def = schema.get_struct(struct_name).ok_or_else(|| {
                CoreError::codec(
                    "decode",
                    format!("struct '{struct_name}' not found in schema"),
                )
            })?;
            let mut fields = Vec::new();
            let mut current_offset = offset;
            for field in &struct_def.fields {
                let (value, consumed) =
                    decode_value_with_endian(data, current_offset, &field.ty, schema, endian)?;
                fields.push((field.name.clone(), value));
                current_offset += consumed;
            }
            Ok((
                DecodedValue::Struct(struct_name.clone(), fields),
                current_offset - offset,
            ))
        }
    }
}

/// 检查缓冲区大小
fn check_buffer_size(data: &[u8], offset: usize, required: usize) -> Result<()> {
    if offset + required > data.len() {
        return Err(CoreError::codec(
            "decode",
            format!(
                "buffer too small: need {} bytes at offset {}, have {}",
                required,
                offset,
                data.len()
            ),
        ));
    }
    Ok(())
}

#[allow(dead_code)]
/// 计算值的大小
fn calculate_value_size(value: &Value, ty: &Type, schema: &Schema) -> Result<usize> {
    match (value, ty) {
        (_, Type::U8 | Type::I8 | Type::Bool) => Ok(1),
        (_, Type::U16 | Type::I16) => Ok(2),
        (_, Type::U32 | Type::I32 | Type::F32) => Ok(4),
        (_, Type::U64 | Type::I64 | Type::F64) => Ok(8),
        (_, Type::U128 | Type::I128) => Ok(16),
        (Value::String(s), Type::String) => Ok(4 + s.len()),
        (Value::Bytes(b), Type::Bytes) => Ok(4 + b.len()),
        // 十六进制整数作为 Bytes（4字节长度前缀 + 整数字节数）
        (Value::Integer(n), Type::Bytes) => {
            // 根据整数大小确定字节数
            let bytes = if *n <= i128::from(u8::MAX) {
                1
            } else if *n <= i128::from(u16::MAX) {
                2
            } else if *n <= i128::from(u32::MAX) {
                4
            } else if *n <= i128::from(u64::MAX) {
                8
            } else {
                16
            };
            Ok(4 + bytes)
        }
        (Value::Array(arr), Type::Vec(inner_ty)) => {
            let mut size = 4; // 长度前缀
            for item in arr {
                size += calculate_value_size(item, inner_ty, schema)?;
            }
            Ok(size)
        }
        (Value::Array(arr), Type::Array(inner_ty, _)) => {
            let mut size = 0;
            for item in arr {
                size += calculate_value_size(item, inner_ty, schema)?;
            }
            Ok(size)
        }
        (Value::Struct(struct_name, fields), Type::Custom(type_name)) => {
            if struct_name != type_name {
                return Err(CoreError::codec(
                    "encode",
                    format!("struct name mismatch: expected {type_name}, got {struct_name}"),
                ));
            }
            let struct_def = schema.get_struct(type_name).ok_or_else(|| {
                CoreError::codec(
                    "encode",
                    format!("struct '{type_name}' not found in schema"),
                )
            })?;
            let mut size = 0;
            for field in &struct_def.fields {
                let field_value = fields
                    .iter()
                    .find(|(name, _)| name == &field.name)
                    .map(|(_, v)| v)
                    .ok_or_else(|| {
                        CoreError::codec(
                            "encode",
                            format!("field '{}' not found in struct init", field.name),
                        )
                    })?;
                size += calculate_value_size(field_value, &field.ty, schema)?;
            }
            Ok(size)
        }
        _ => Err(CoreError::codec(
            "encode",
            format!("cannot calculate size for value {value:?} with type {ty:?}"),
        )),
    }
}

/// 计算类型的大小（不包括动态大小类型）
fn calculate_type_size(ty: &Type, schema: &Schema) -> Result<usize> {
    match ty {
        Type::U8 | Type::I8 | Type::Bool => Ok(1),
        Type::U16 | Type::I16 => Ok(2),
        Type::U32 | Type::I32 | Type::F32 => Ok(4),
        Type::U64 | Type::I64 | Type::F64 => Ok(8),
        Type::U128 | Type::I128 => Ok(16),
        Type::String | Type::Bytes => Err(CoreError::codec(
            "encode",
            format!("type {ty:?} has dynamic size"),
        )),
        Type::Vec(_) => Err(CoreError::codec(
            "encode",
            format!("type {ty:?} has dynamic size"),
        )),
        Type::Array(inner_ty, size) => {
            let inner_size = calculate_type_size(inner_ty, schema)?;
            Ok(inner_size * size)
        }
        Type::Custom(struct_name) => {
            let struct_def = schema.get_struct(struct_name).ok_or_else(|| {
                CoreError::codec(
                    "encode",
                    format!("struct '{struct_name}' not found in schema"),
                )
            })?;
            let mut size = 0;
            for field in &struct_def.fields {
                size += calculate_type_size(&field.ty, schema)?;
            }
            Ok(size)
        }
    }
}

#[allow(dead_code)]
/// 获取文件级字节序属性
fn get_file_endian_attribute(schema: &Schema) -> Option<Endianness> {
    schema.file_attributes.iter().find_map(|attr| match attr {
        crate::ast::FileAttribute::Endian(endian) => Some(*endian),
        _ => None,
    })
}

/// 判断类型是否为变长类型（Vec/String/Bytes）
fn is_variable_length_type(ty: &Type) -> bool {
    matches!(ty, Type::Vec(_) | Type::String | Type::Bytes)
}

/// 从已解码字段中获取整数值（转为 usize）
fn get_decoded_field_value_as_usize(
    fields: &[(String, DecodedValue)],
    name: &str,
) -> Option<usize> {
    fields.iter().find_map(|(n, v)| {
        if n != name {
            return None;
        }
        match v {
            DecodedValue::U8(val) => Some(*val as usize),
            DecodedValue::U16(val) => Some(*val as usize),
            DecodedValue::U32(val) => Some(*val as usize),
            DecodedValue::U64(val) => Some(*val as usize),
            DecodedValue::I8(val) if *val >= 0 => Some(*val as usize),
            DecodedValue::I16(val) if *val >= 0 => Some(*val as usize),
            DecodedValue::I32(val) if *val >= 0 => Some(*val as usize),
            DecodedValue::I64(val) if *val >= 0 => Some(*val as usize),
            _ => None,
        }
    })
}

/// 计算整数作为 Bytes 类型时的字节长度
fn integer_to_byte_len(n: i128) -> u64 {
    if n <= i128::from(u8::MAX) {
        1
    } else if n <= i128::from(u16::MAX) {
        2
    } else if n <= i128::from(u32::MAX) {
        4
    } else if n <= i128::from(u64::MAX) {
        8
    } else {
        16
    }
}
