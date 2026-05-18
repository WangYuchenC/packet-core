//! 集成测试

use packet_core::{parse_schema, validate_schema, Codec, DecodedValue};

#[test]
fn test_sensor_data_roundtrip() {
    let schema_src = r#"
        #[send]
        struct SensorDataSend {
            device_id: u32 = 0x12345678,
            temperature: f32 = 25.5,
            humidity: u8 = 60,
            active: bool = true,
        }

        #[receive]
        struct SensorDataRecv {
            device_id: u32,
            temperature: f32,
            humidity: u8,
            active: bool,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "SensorDataSend").expect("compile send codec failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 10);

    assert_eq!(&encoded[0..4], &[0x12, 0x34, 0x56, 0x78]);
    assert_eq!(&encoded[4..8], &[0x41, 0xCC, 0x00, 0x00]);
    assert_eq!(encoded[8], 0x3C);
    assert_eq!(encoded[9], 0x01);

    let recv_codec = Codec::compile(&schema, "SensorDataRecv").expect("compile recv codec failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.name, "SensorDataRecv");
    assert_eq!(decoded.fields.len(), 4);
    assert_eq!(decoded.fields[0].1, DecodedValue::U32(0x12345678));
    assert_eq!(decoded.fields[1].1, DecodedValue::F32(25.5));
    assert_eq!(decoded.fields[2].1, DecodedValue::U8(60));
    assert_eq!(decoded.fields[3].1, DecodedValue::Bool(true));
}

#[test]
fn test_string_and_bytes_roundtrip() {
    let schema_src = r#"
        #[send]
        struct MessageSend {
            id: u32 = 1,
            content: String = "Hello, World!",
            data: Bytes = 0xDEADBEEF,
        }

        #[receive]
        struct MessageRecv {
            id: u32,
            content: String,
            data: Bytes,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "MessageSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(&encoded[0..4], &[0x00, 0x00, 0x00, 0x01]);
    assert_eq!(&encoded[4..8], &[0x00, 0x00, 0x00, 0x0D]);
    assert_eq!(&encoded[8..21], b"Hello, World!");
    assert_eq!(&encoded[21..25], &[0x00, 0x00, 0x00, 0x04]);
    assert_eq!(&encoded[25..29], &[0xDE, 0xAD, 0xBE, 0xEF]);

    let recv_codec = Codec::compile(&schema, "MessageRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 3);
    assert_eq!(decoded.fields[0].1, DecodedValue::U32(1));
    assert_eq!(
        decoded.fields[1].1,
        DecodedValue::String("Hello, World!".to_string())
    );
    assert_eq!(
        decoded.fields[2].1,
        DecodedValue::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF])
    );
}

#[test]
fn test_array_roundtrip() {
    let schema_src = r#"
        #[send]
        struct ArrayDataSend {
            values: [u16; 4] = [1, 2, 3, 4],
            flags: [bool; 8] = [true, false, true, false, true, false, true, false],
        }

        #[receive]
        struct ArrayDataRecv {
            values: [u16; 4],
            flags: [bool; 8],
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "ArrayDataSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 16);

    assert_eq!(&encoded[0..2], &[0x00, 0x01]);
    assert_eq!(&encoded[2..4], &[0x00, 0x02]);
    assert_eq!(&encoded[4..6], &[0x00, 0x03]);
    assert_eq!(&encoded[6..8], &[0x00, 0x04]);
    assert_eq!(
        &encoded[8..16],
        &[0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01, 0x00]
    );

    let recv_codec = Codec::compile(&schema, "ArrayDataRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    if let DecodedValue::Vec(values) = &decoded.fields[0].1 {
        assert_eq!(values.len(), 4);
        assert_eq!(values[0], DecodedValue::U16(1));
        assert_eq!(values[1], DecodedValue::U16(2));
        assert_eq!(values[2], DecodedValue::U16(3));
        assert_eq!(values[3], DecodedValue::U16(4));
    } else {
        panic!("expected Vec for values field");
    }
}

#[test]
fn test_vec_roundtrip() {
    let schema_src = r#"
        #[send]
        struct VecDataSend {
            count: u32 = 3,
            items: Vec<u32> = [10, 20, 30],
        }

        #[receive]
        struct VecDataRecv {
            count: u32,
            items: Vec<u32>,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "VecDataSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");
    assert_eq!(encoded.len(), 20);

    assert_eq!(&encoded[0..4], &[0x00, 0x00, 0x00, 0x03]);
    assert_eq!(&encoded[4..8], &[0x00, 0x00, 0x00, 0x03]);
    assert_eq!(&encoded[8..12], &[0x00, 0x00, 0x00, 0x0A]);
    assert_eq!(&encoded[12..16], &[0x00, 0x00, 0x00, 0x14]);
    assert_eq!(&encoded[16..20], &[0x00, 0x00, 0x00, 0x1E]);

    let recv_codec = Codec::compile(&schema, "VecDataRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U32(3));
    if let DecodedValue::Vec(items) = &decoded.fields[1].1 {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], DecodedValue::U32(10));
        assert_eq!(items[1], DecodedValue::U32(20));
        assert_eq!(items[2], DecodedValue::U32(30));
    } else {
        panic!("expected Vec for items field");
    }
}

#[test]
fn test_empty_struct() {
    let schema_src = r#"
        #[send]
        struct EmptySend {
        }

        #[receive]
        struct EmptyRecv {
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "EmptySend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");
    assert!(encoded.is_empty());

    let recv_codec = Codec::compile(&schema, "EmptyRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");
    assert!(decoded.fields.is_empty());
}

#[test]
fn test_multiple_structs() {
    let schema_src = r#"
        #[send]
        struct HeaderSend {
            magic: u32 = 0xDEADBEEF,
            version: u16 = 1,
        }

        #[send]
        struct BodySend {
            data: [u8; 4] = [1, 2, 3, 4],
        }

        #[receive]
        struct HeaderRecv {
            magic: u32,
            version: u16,
        }

        #[receive]
        struct BodyRecv {
            data: [u8; 4],
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let header_send = Codec::compile(&schema, "HeaderSend").expect("compile failed");
    let header_encoded = header_send.encode().expect("encode failed");
    assert_eq!(header_encoded.len(), 6);

    let body_send = Codec::compile(&schema, "BodySend").expect("compile failed");
    let body_encoded = body_send.encode().expect("encode failed");
    assert_eq!(body_encoded.len(), 4);
}

#[test]
fn test_parse_error_invalid_syntax() {
    let schema_src = "struct @invalid";
    let result = parse_schema(schema_src);
    assert!(result.is_err());
}

#[test]
fn test_validation_error_duplicate_struct() {
    let schema_src = r#"
        #[send]
        struct Test { value: u32 = 1, }
        #[send]
        struct Test { value: u32 = 2, }
    "#;

    let schema = parse_schema(schema_src).expect("parse should succeed");
    let result = validate_schema(&schema);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("duplicate"));
}

#[test]
fn test_validation_error_type_mismatch() {
    let schema_src = r#"
        #[send]
        struct Test { value: bool = 42, }
    "#;

    let schema = parse_schema(schema_src).expect("parse should succeed");
    let result = validate_schema(&schema);
    assert!(result.is_err());
}

#[test]
fn test_codec_compile_error_not_found() {
    let schema_src = r#"
        #[send]
        struct Existing { value: u32 = 1, }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let result = Codec::compile(&schema, "NonExistent");
    assert!(result.is_err());
}

#[test]
fn test_codec_encode_error_not_send() {
    let schema_src = r#"
        #[receive]
        struct Test { value: u32, }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let codec = Codec::compile(&schema, "Test").expect("compile failed");
    let result = codec.encode();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not a send type"));
}

#[test]
fn test_codec_decode_error_not_receive() {
    let schema_src = r#"
        #[send]
        struct Test { value: u32 = 1, }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let codec = Codec::compile(&schema, "Test").expect("compile failed");
    let result = codec.decode(&[0, 0, 0, 1]);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not a receive type"));
}

#[test]
fn test_codec_symmetry() {
    let schema_src = r#"
        #[send]
        struct AllTypesSend {
            u8_val: u8 = 1,
            u16_val: u16 = 2,
            u32_val: u32 = 3,
            u64_val: u64 = 4,
            u128_val: u128 = 5,
            i8_val: i8 = -1,
            i16_val: i16 = -2,
            i32_val: i32 = -3,
            i64_val: i64 = -4,
            i128_val: i128 = -5,
            f32_val: f32 = 1.5,
            f64_val: f64 = 2.5,
            bool_val: bool = true,
        }

        #[receive]
        struct AllTypesRecv {
            u8_val: u8,
            u16_val: u16,
            u32_val: u32,
            u64_val: u64,
            u128_val: u128,
            i8_val: i8,
            i16_val: i16,
            i32_val: i32,
            i64_val: i64,
            i128_val: i128,
            f32_val: f32,
            f64_val: f64,
            bool_val: bool,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "AllTypesSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    let recv_codec = Codec::compile(&schema, "AllTypesRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 13);

    assert_eq!(decoded.fields[0].1, DecodedValue::U8(1));
    assert_eq!(decoded.fields[1].1, DecodedValue::U16(2));
    assert_eq!(decoded.fields[2].1, DecodedValue::U32(3));
    assert_eq!(decoded.fields[3].1, DecodedValue::U64(4));
    assert_eq!(decoded.fields[4].1, DecodedValue::U128(5));
    assert_eq!(decoded.fields[5].1, DecodedValue::I8(-1));
    assert_eq!(decoded.fields[6].1, DecodedValue::I16(-2));
    assert_eq!(decoded.fields[7].1, DecodedValue::I32(-3));
    assert_eq!(decoded.fields[8].1, DecodedValue::I64(-4));
    assert_eq!(decoded.fields[9].1, DecodedValue::I128(-5));
    assert_eq!(decoded.fields[10].1, DecodedValue::F32(1.5));
    assert_eq!(decoded.fields[11].1, DecodedValue::F64(2.5));
    assert_eq!(decoded.fields[12].1, DecodedValue::Bool(true));
}

#[test]
fn test_boundary_max_values() {
    let schema_src = r#"
        #[send]
        struct MaxValuesSend {
            u8_max: u8 = 255,
            u16_max: u16 = 65535,
            u32_max: u32 = 4294967295,
            u64_max: u64 = 18446744073709551615,
        }

        #[receive]
        struct MaxValuesRecv {
            u8_max: u8,
            u16_max: u16,
            u32_max: u32,
            u64_max: u64,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let send_codec = Codec::compile(&schema, "MaxValuesSend").expect("compile failed");
    let _encoded = send_codec.encode().expect("encode failed");
}

#[test]
fn test_boundary_min_values() {
    let schema_src = r#"
        #[send]
        struct MinValuesSend {
            i8_min: i8 = -128,
            i16_min: i16 = -32768,
            i32_min: i32 = -2147483648,
            i64_min: i64 = -9223372036854775808,
        }

        #[receive]
        struct MinValuesRecv {
            i8_min: i8,
            i16_min: i16,
            i32_min: i32,
            i64_min: i64,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let send_codec = Codec::compile(&schema, "MinValuesSend").expect("compile failed");
    let _encoded = send_codec.encode().expect("encode failed");
}

#[test]
fn test_schema_with_comments() {
    let schema_src = r#"
        // This is a header comment
        #[send]
        struct TestSend {
            // This is a field comment
            value: u32 = 42, // inline comment
        }

        /*
         * Multi-line comment
         */
        #[receive]
        struct TestRecv {
            value: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "TestSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    let recv_codec = Codec::compile(&schema, "TestRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 1);
    assert_eq!(decoded.fields[0].1, DecodedValue::U32(42));
}

// ==================== Auto 和 LenRef 特性测试 ====================

#[test]
fn test_auto_field_length() {
    let schema_src = r#"
        #[send]
        struct AutoLenSend {
            #[auto]
            total_len: u32,
            header: u32 = 0x12345678,
            payload: [u8; 4] = [1, 2, 3, 4],
        }

        #[receive]
        struct AutoLenRecv {
            total_len: u32,
            header: u32,
            payload: [u8; 4],
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "AutoLenSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(&encoded[0..4], &[0x00, 0x00, 0x00, 0x0C]);
    assert_eq!(&encoded[4..8], &[0x12, 0x34, 0x56, 0x78]);
    assert_eq!(&encoded[8..12], &[0x01, 0x02, 0x03, 0x04]);

    let recv_codec = Codec::compile(&schema, "AutoLenRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields[0].1, DecodedValue::U32(12));
    assert_eq!(decoded.fields[1].1, DecodedValue::U32(0x12345678));
}

#[test]
fn test_auto_field_with_reference() {
    let schema_src = r#"
        #[send]
        struct AutoRefSend {
            #[auto(items)]
            count: u8,
            items: Vec<u32> = [100, 200, 300],
        }

        #[receive]
        struct AutoRefRecv {
            count: u8,
            items: Vec<u32>,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "AutoRefSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded[0], 0x03);

    let recv_codec = Codec::compile(&schema, "AutoRefRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields[0].1, DecodedValue::U8(3));
    if let DecodedValue::Vec(items) = &decoded.fields[1].1 {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], DecodedValue::U32(100));
        assert_eq!(items[1], DecodedValue::U32(200));
        assert_eq!(items[2], DecodedValue::U32(300));
    } else {
        panic!("expected Vec");
    }
}

#[test]
fn test_len_ref_field_decode() {
    let schema_src = r#"
        #[send]
        struct LenRefDataSend {
            count: u8 = 3,
            items: Vec<u8> = [10, 20, 30],
        }

        #[receive]
        struct LenRefDataRecv {
            count: u8,
            #[len_ref(count)]
            items: Vec<u8>,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "LenRefDataSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded[0], 0x03);
    assert_eq!(&encoded[1..5], &[0x00, 0x00, 0x00, 0x03]);
    assert_eq!(&encoded[5..8], &[0x0A, 0x14, 0x1E]);

    let recv_codec = Codec::compile(&schema, "LenRefDataRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U8(3));
    if let DecodedValue::Vec(items) = &decoded.fields[1].1 {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], DecodedValue::U8(10));
        assert_eq!(items[1], DecodedValue::U8(20));
        assert_eq!(items[2], DecodedValue::U8(30));
    } else {
        panic!("expected Vec");
    }
}

// ==================== #[remaining] 测试 ====================

#[test]
fn test_remaining_field_bytes() {
    let schema_src = r#"
        #[send]
        struct SimpleHeaderSend {
            header: u32 = 0x12345678,
        }

        #[receive]
        struct RemainingBytesRecv {
            header: u32,
            #[remaining]
            payload: Bytes,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "SimpleHeaderSend").expect("compile failed");
    let mut encoded = send_codec.encode().expect("encode failed");
    encoded.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);

    let recv_codec = Codec::compile(&schema, "RemainingBytesRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U32(0x12345678));
    assert_eq!(
        decoded.fields[1].1,
        DecodedValue::Bytes(vec![0x01, 0x02, 0x03, 0x04])
    );
}

#[test]
fn test_remaining_field_vec_u8() {
    let schema_src = r#"
        #[send]
        struct CountOnlySend {
            count: u8 = 3,
        }

        #[receive]
        struct RemainingVecRecv {
            count: u8,
            #[remaining]
            items: Vec<u8>,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "CountOnlySend").expect("compile failed");
    let mut encoded = send_codec.encode().expect("encode failed");
    encoded.extend_from_slice(&[0x0A, 0x0B, 0x0C]);

    let recv_codec = Codec::compile(&schema, "RemainingVecRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U8(3));
    if let DecodedValue::Vec(items) = &decoded.fields[1].1 {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], DecodedValue::U8(0x0A));
        assert_eq!(items[1], DecodedValue::U8(0x0B));
        assert_eq!(items[2], DecodedValue::U8(0x0C));
    } else {
        panic!("expected Vec for items field");
    }
}

// ==================== 位域测试 ====================

#[test]
fn test_bitfield_encode_decode() {
    let schema_src = r#"
        #[send]
        struct StatusSend {
            #[bits(0, 3)]
            code: u8 = 5,
            #[bit(4)]
            active: bool = true,
            #[bits(5, 7)]
            mode: u8 = 2,
        }

        #[receive]
        struct StatusRecv {
            #[bits(0, 3)]
            code: u8,
            #[bit(4)]
            active: bool,
            #[bits(5, 7)]
            mode: u8,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "StatusSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 1);
    assert_eq!(encoded[0], 85);

    let recv_codec = Codec::compile(&schema, "StatusRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 3);
    assert_eq!(decoded.fields[0].1, DecodedValue::U64(5));
    assert_eq!(decoded.fields[1].1, DecodedValue::Bool(true));
    assert_eq!(decoded.fields[2].1, DecodedValue::U64(2));
}

// ==================== 条件字段测试 ====================

#[test]
fn test_conditional_field_true() {
    let schema_src = r#"
        #[send]
        struct CondTrueSend {
            has_data: bool = true,
            #[if(has_data)]
            data: u32 = 100,
        }

        #[receive]
        struct CondTrueRecv {
            has_data: bool,
            #[if(has_data)]
            data: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "CondTrueSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 5);
    assert_eq!(encoded[0], 0x01);
    assert_eq!(&encoded[1..5], &[0x00, 0x00, 0x00, 0x64]);

    let recv_codec = Codec::compile(&schema, "CondTrueRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::Bool(true));
    assert_eq!(decoded.fields[1].1, DecodedValue::U32(100));
}

#[test]
fn test_conditional_field_false() {
    let schema_src = r#"
        #[send]
        struct CondFalseSend {
            has_data: bool = false,
            #[if(has_data)]
            data: u32 = 100,
        }

        #[receive]
        struct CondFalseRecv {
            has_data: bool,
            #[if(has_data)]
            data: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "CondFalseSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 1);
    assert_eq!(encoded[0], 0x00);

    let recv_codec = Codec::compile(&schema, "CondFalseRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 1);
    assert_eq!(decoded.fields[0].1, DecodedValue::Bool(false));
}

// ==================== 校验和测试 ====================

#[test]
fn test_checksum_crc32() {
    let schema_src = r#"
        #[send]
        struct Crc32Send {
            data: u16 = 0x1234,
            #[checksum(crc32)]
            crc: u32 = 0,
        }

        #[receive]
        struct Crc32Recv {
            data: u16,
            crc: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "Crc32Send").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 6);
    assert_eq!(&encoded[0..2], &[0x12, 0x34]);
    assert_ne!(&encoded[2..6], &[0x00, 0x00, 0x00, 0x00]);

    let recv_codec = Codec::compile(&schema, "Crc32Recv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U16(0x1234));
    let crc_val = if let DecodedValue::U32(v) = decoded.fields[1].1 {
        v
    } else {
        panic!("expected U32")
    };
    assert_ne!(crc_val, 0);
}

#[test]
fn test_checksum_xor() {
    let schema_src = r#"
        #[send]
        struct XorSend {
            data: u32 = 0x12345678,
            #[checksum(xor)]
            checksum: u8 = 0,
        }

        #[receive]
        struct XorRecv {
            data: u32,
            checksum: u8,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "XorSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 5);
    assert_eq!(&encoded[0..4], &[0x12, 0x34, 0x56, 0x78]);
    assert_eq!(encoded[4], 0x08);

    let recv_codec = Codec::compile(&schema, "XorRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields[0].1, DecodedValue::U32(0x12345678));
    assert_eq!(decoded.fields[1].1, DecodedValue::U8(0x08));
}

// ==================== 字节序测试 ====================

#[test]
fn test_field_level_endian() {
    let schema_src = r#"
        #[send]
        struct MixedEndianSend {
            field_a: u32 = 0x12345678,
            #[endian(little)]
            field_b: u32 = 0x12345678,
        }

        #[receive]
        struct MixedEndianRecv {
            field_a: u32,
            #[endian(little)]
            field_b: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "MixedEndianSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(&encoded[0..4], &[0x12, 0x34, 0x56, 0x78]);
    assert_eq!(&encoded[4..8], &[0x78, 0x56, 0x34, 0x12]);

    let recv_codec = Codec::compile(&schema, "MixedEndianRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U32(0x12345678));
    assert_eq!(decoded.fields[1].1, DecodedValue::U32(0x12345678));
}

// ==================== 嵌套结构体测试 ====================

#[test]
fn test_nested_struct_roundtrip() {
    let schema_src = r#"
        struct Address {
            street: String = "123 Main St",
            city: String = "Springfield",
        }

        #[send]
        struct PersonSend {
            name: String = "Alice",
            age: u8 = 30,
            address: Address = Address { street: "123 Main St", city: "Springfield" },
        }

        #[receive]
        struct PersonRecv {
            name: String,
            age: u8,
            address: Address,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "PersonSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    let recv_codec = Codec::compile(&schema, "PersonRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 3);
    assert_eq!(
        decoded.fields[0].1,
        DecodedValue::String("Alice".to_string())
    );
    assert_eq!(decoded.fields[1].1, DecodedValue::U8(30));

    if let DecodedValue::Struct(name, fields) = &decoded.fields[2].1 {
        assert_eq!(name, "Address");
        assert_eq!(fields.len(), 2);
        assert_eq!(
            fields[0],
            (
                "street".to_string(),
                DecodedValue::String("123 Main St".to_string())
            )
        );
        assert_eq!(
            fields[1],
            (
                "city".to_string(),
                DecodedValue::String("Springfield".to_string())
            )
        );
    } else {
        panic!("expected Struct for address field");
    }
}

// ==================== 错误路径测试 ====================

#[test]
fn test_decode_buffer_too_small() {
    let schema_src = r#"
        #[receive]
        struct NeedMoreData {
            value: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let codec = Codec::compile(&schema, "NeedMoreData").expect("compile failed");

    let result = codec.decode(&[0x00, 0x01]);
    assert!(result.is_err());
}

#[test]
fn test_decode_trailing_data() {
    let schema_src = r#"
        #[receive]
        struct ExactSize {
            value: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let codec = Codec::compile(&schema, "ExactSize").expect("compile failed");

    let result = codec.decode(&[0x00, 0x00, 0x00, 0x01, 0xFF, 0xFF]);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("trailing"));
}

#[test]
fn test_encode_field_no_value() {
    let schema_src = r#"
        #[send]
        struct MissingValue {
            value: u32,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");

    let result = validate_schema(&schema);
    assert!(result.is_err());
}

// ==================== #[prefix(disable)] 测试 ====================

#[test]
fn test_prefix_disabled_vec_roundtrip() {
    let schema_src = r#"
        #![prefix(disable)]

        #[send]
        struct ManualLenSend {
            #[auto(data)]
            count: u8,
            data: Vec<u8> = [10, 20, 30],
        }

        #[receive]
        struct ManualLenRecv {
            count: u8,
            #[len_ref(count)]
            data: Vec<u8>,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "ManualLenSend").expect("compile send failed");
    let encoded = send_codec.encode().expect("encode failed");

    // count(1) + 3 data bytes = 4 bytes total, no 4-byte Vec prefix
    assert_eq!(encoded.len(), 4);
    assert_eq!(encoded[0], 0x03);          // auto-filled count
    assert_eq!(&encoded[1..4], &[0x0A, 0x14, 0x1E]); // data bytes

    let recv_codec = Codec::compile(&schema, "ManualLenRecv").expect("compile recv failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U8(3));
    if let DecodedValue::Vec(items) = &decoded.fields[1].1 {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], DecodedValue::U8(10));
        assert_eq!(items[1], DecodedValue::U8(20));
        assert_eq!(items[2], DecodedValue::U8(30));
    } else {
        panic!("expected Vec for data field");
    }
}

#[test]
fn test_prefix_disabled_vec_u32_roundtrip() {
    let schema_src = r#"
        #![prefix(disable)]

        #[send]
        struct VecU32Send {
            #[auto(items)]
            count: u16,
            items: Vec<u32> = [100, 200, 300],
        }

        #[receive]
        struct VecU32Recv {
            count: u16,
            #[len_ref(count)]
            items: Vec<u32>,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "VecU32Send").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    // count(2) + 3*4 = 14 bytes, no 4-byte Vec prefix
    assert_eq!(encoded.len(), 14);
    assert_eq!(&encoded[0..2], &[0x00, 0x03]); // count = 3
    assert_eq!(&encoded[2..6], &[0x00, 0x00, 0x00, 0x64]); // 100
    assert_eq!(&encoded[6..10], &[0x00, 0x00, 0x00, 0xC8]); // 200
    assert_eq!(&encoded[10..14], &[0x00, 0x00, 0x01, 0x2C]); // 300

    let recv_codec = Codec::compile(&schema, "VecU32Recv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U16(3));
    if let DecodedValue::Vec(items) = &decoded.fields[1].1 {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], DecodedValue::U32(100));
        assert_eq!(items[1], DecodedValue::U32(200));
        assert_eq!(items[2], DecodedValue::U32(300));
    } else {
        panic!("expected Vec for items field");
    }
}

#[test]
fn test_prefix_disabled_bytes_roundtrip() {
    let schema_src = r#"
        #![prefix(disable)]

        #[send]
        struct BytesSend {
            #[auto(payload)]
            len: u8,
            payload: Bytes = 0xBEAD,
        }

        #[receive]
        struct BytesRecv {
            len: u8,
            #[len_ref(len)]
            payload: Bytes,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "BytesSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    // len(1) + 2 data bytes = 3 bytes, no 4-byte Bytes prefix
    assert_eq!(encoded.len(), 3);
    assert_eq!(encoded[0], 0x02);
    assert_eq!(&encoded[1..3], &[0xBE, 0xAD]);

    let recv_codec = Codec::compile(&schema, "BytesRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U8(2));
    assert_eq!(
        decoded.fields[1].1,
        DecodedValue::Bytes(vec![0xBE, 0xAD])
    );
}

#[test]
fn test_prefix_disabled_string_roundtrip() {
    let schema_src = r#"
        #![prefix(disable)]

        #[send]
        struct StringSend {
            #[auto(text)]
            count: u8,
            text: String = "ABC",
        }

        #[receive]
        struct StringRecv {
            count: u8,
            #[len_ref(count)]
            text: String,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "StringSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    assert_eq!(encoded.len(), 4);
    assert_eq!(encoded[0], 0x03);
    assert_eq!(&encoded[1..4], b"ABC");

    let recv_codec = Codec::compile(&schema, "StringRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U8(3));
    assert_eq!(
        decoded.fields[1].1,
        DecodedValue::String("ABC".to_string())
    );
}

#[test]
fn test_prefix_disabled_with_remaining() {
    let schema_src = r#"
        #![prefix(disable)]

        #[send]
        struct HeaderSend {
            magic: u32 = 0x12345678,
        }

        #[receive]
        struct HeaderWithRemainingRecv {
            magic: u32,
            #[remaining]
            payload: Bytes,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "HeaderSend").expect("compile failed");
    let mut encoded = send_codec.encode().expect("encode failed");
    encoded.extend_from_slice(&[0x01, 0x02, 0x03]);

    let recv_codec =
        Codec::compile(&schema, "HeaderWithRemainingRecv").expect("compile failed");
    let decoded = recv_codec.decode(&encoded).expect("decode failed");

    assert_eq!(decoded.fields.len(), 2);
    assert_eq!(decoded.fields[0].1, DecodedValue::U32(0x12345678));
    assert_eq!(
        decoded.fields[1].1,
        DecodedValue::Bytes(vec![0x01, 0x02, 0x03])
    );
}

#[test]
fn test_prefix_disabled_validator_rejects_missing_len_ref() {
    let schema_src = r#"
        #![prefix(disable)]

        #[receive]
        struct BadRecv {
            data: Vec<u8>,  // no len_ref, no remaining — should fail validation
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    let result = validate_schema(&schema);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("E016"));
}

#[test]
fn test_prefix_disabled_and_prefix_enabled_coexist() {
    // Verify that without prefix(disable), the old behavior is preserved
    let schema_src = r#"
        #[send]
        struct NormalSend {
            data: Vec<u8> = [1, 2, 3],
        }

        #[receive]
        struct NormalRecv {
            data: Vec<u8>,
        }
    "#;

    let schema = parse_schema(schema_src).expect("parse failed");
    validate_schema(&schema).expect("validation failed");

    let send_codec = Codec::compile(&schema, "NormalSend").expect("compile failed");
    let encoded = send_codec.encode().expect("encode failed");

    // Normal prefix should still be present
    assert_eq!(encoded.len(), 7);
    assert_eq!(&encoded[0..4], &[0x00, 0x00, 0x00, 0x03]);
    assert_eq!(&encoded[4..7], &[0x01, 0x02, 0x03]);
}
