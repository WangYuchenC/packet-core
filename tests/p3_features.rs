//! P3 文件级特性集成测试

use packet_core::{
    ast::{Endianness, FileAttribute},
    loader::load_schema,
    Codec,
};
use std::io::Write;
use tempfile::NamedTempFile;

/// 测试完整的P3功能链路
#[test]
fn test_full_p3_feature_integration() {
    // 创建被导入的公共文件
    let mut common_file = NamedTempFile::with_suffix(".pkt").unwrap();
    writeln!(
        common_file,
        r#"
        /// Common header for all packets
        #[send]
        struct CommonHeader {{
            /// Magic number
            magic: u16 = 0xABCD,
            /// Packet length
            length: u8 = 4,
        }}
    "#
    )
    .unwrap();

    let common_path = common_file.path().to_path_buf();
    let common_name = common_path.file_name().unwrap().to_str().unwrap();

    // 创建主文件，包含所有P3特性
    let mut main_file = NamedTempFile::with_suffix(".pkt").unwrap();
    writeln!(
        main_file,
        r#"
        #![version("2.0.0")]
        #![endian(little)]

        /// Import common packet types
        import("{}")

        /// Sensor data packet
        /// Contains temperature readings
        #[send]
        struct SensorPacket {{
            /// Header
            header: u32 = 0x12345678,
            /// Temperature value
            temp: f64 = 25.5,
            /// Status flag
            status: u8 = 1,
        }}
    "#,
        common_name
    )
    .unwrap();

    // 加载schema
    let schema = load_schema(main_file.path()).unwrap();

    // 验证文件级属性
    assert_eq!(schema.file_attributes.len(), 2);
    match &schema.file_attributes[0] {
        FileAttribute::Version(v) => assert_eq!(v, "2.0.0"),
        _ => panic!("Expected Version attribute"),
    }
    match &schema.file_attributes[1] {
        FileAttribute::Endian(e) => assert_eq!(*e, Endianness::Little),
        _ => panic!("Expected Endian attribute"),
    }

    // 验证导入
    assert_eq!(schema.imports.len(), 1);
    assert_eq!(schema.imports[0].path, common_name);

    // 验证文档注释
    let sensor = schema.get_struct("SensorPacket").unwrap();
    assert!(sensor.doc.is_some());
    let doc = sensor.doc.as_ref().unwrap();
    assert!(doc.contains("Sensor data packet"));
    assert!(doc.contains("temperature readings"));

    // 验证导入的结构体存在
    assert!(schema.get_struct("CommonHeader").is_some());

    // 验证文件级字节序在Codec中生效
    let codec = Codec::compile(&schema, "SensorPacket").unwrap();
    let encoded = codec.encode().unwrap();

    // 小端序: 0x78 0x56 0x34 0x12
    assert_eq!(&encoded[0..4], &[0x78, 0x56, 0x34, 0x12]);
}

/// 测试Schema default_endian方法
#[test]
fn test_schema_default_endian() {
    use packet_core::ast::Schema;

    let mut schema = Schema::new();
    assert_eq!(schema.default_endian(), Endianness::Big); // 默认大端

    schema
        .file_attributes
        .push(FileAttribute::Endian(Endianness::Little));
    assert_eq!(schema.default_endian(), Endianness::Little);
}

/// 测试import加载后属性合并
#[test]
fn test_import_file_level_endian_applied() {
    // 创建子文件，定义小端序
    let mut sub_file = NamedTempFile::with_suffix(".pkt").unwrap();
    writeln!(
        sub_file,
        r#"
        #![endian(little)]

        #[send]
        struct LittleEndianData {{
            value: u32 = 0x12345678,
        }}
    "#
    )
    .unwrap();

    let schema = load_schema(sub_file.path()).unwrap();
    assert_eq!(schema.file_attributes.len(), 1);

    // 验证Codec使用文件级字节序
    let codec = Codec::compile(&schema, "LittleEndianData").unwrap();
    let encoded = codec.encode().unwrap();

    // 小端序: 0x78 0x56 0x34 0x12
    assert_eq!(encoded, vec![0x78, 0x56, 0x34, 0x12]);
}

/// 测试没有文件级属性时使用默认大端序
#[test]
fn test_default_big_endian_without_file_attr() {
    let mut file = NamedTempFile::with_suffix(".pkt").unwrap();
    writeln!(
        file,
        r#"
        #[send]
        struct BigEndianData {{
            value: u32 = 0x12345678,
        }}
    "#
    )
    .unwrap();

    let schema = load_schema(file.path()).unwrap();
    assert_eq!(schema.default_endian(), Endianness::Big);

    let codec = Codec::compile(&schema, "BigEndianData").unwrap();
    let encoded = codec.encode().unwrap();

    // 大端序: 0x12 0x34 0x56 0x78
    assert_eq!(encoded, vec![0x12, 0x34, 0x56, 0x78]);
}
