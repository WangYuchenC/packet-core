//! Codec 编解码性能基准测试

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use packet_core::{parse_schema, Codec};

fn benchmark_codec_encode(c: &mut Criterion) {
    let schema = parse_schema(
        r#"
        #[send]
        struct EncodeOnly {
            id: u32 = 0x12345678,
            temperature: f32 = 25.5,
            humidity: u8 = 60,
            name: String = "sensor_data",
            payload: Vec<u8> = [0x01, 0x02, 0x03, 0x04, 0x05],
        }
    "#,
    )
    .expect("Failed to parse schema");

    let codec = Codec::compile(&schema, "EncodeOnly").unwrap();

    c.bench_function("codec/encode/simple", |b| {
        b.iter(|| {
            black_box(&codec).encode().unwrap();
        });
    });
}

fn benchmark_codec_decode(c: &mut Criterion) {
    // First create a send-only schema to generate test data
    let send_schema = parse_schema(
        r#"
        #[send]
        struct DecodeTest {
            id: u32 = 0x12345678,
            temperature: f32 = 25.5,
            humidity: u8 = 60,
            name: String = "sensor_data",
            payload: Vec<u8> = [0x01, 0x02, 0x03, 0x04, 0x05],
        }
    "#,
    )
    .expect("Failed to parse send schema");

    let send_codec = Codec::compile(&send_schema, "DecodeTest").unwrap();
    let encoded = send_codec.encode().unwrap();

    // Now create receive schema for decoding
    let recv_schema = parse_schema(
        r#"
        #[receive]
        struct DecodeTest {
            id: u32,
            temperature: f32,
            humidity: u8,
            name: String,
            payload: Vec<u8>,
        }
    "#,
    )
    .expect("Failed to parse receive schema");

    let recv_codec = Codec::compile(&recv_schema, "DecodeTest").unwrap();

    c.bench_function("codec/decode/simple", |b| {
        b.iter(|| {
            black_box(&recv_codec).decode(black_box(&encoded)).unwrap();
        });
    });
}

fn benchmark_codec_large_payload(c: &mut Criterion) {
    let send_schema = parse_schema(
        r#"
        #[send]
        struct LargePayload {
            header: u32 = 0xDEADBEEF,
            data: Bytes = 10000,
            footer: u32 = 0xCAFEBABE,
        }
    "#,
    )
    .expect("Failed to parse send schema");

    let recv_schema = parse_schema(
        r#"
        #[receive]
        struct LargePayload {
            header: u32,
            data: Bytes = 10000,
            footer: u32,
        }
    "#,
    )
    .expect("Failed to parse receive schema");

    let send_codec = Codec::compile(&send_schema, "LargePayload").unwrap();
    let encoded = send_codec.encode().unwrap();

    let recv_codec = Codec::compile(&recv_schema, "LargePayload").unwrap();

    c.bench_function("codec/encode/large_payload", |b| {
        b.iter(|| {
            black_box(&send_codec).encode().unwrap();
        });
    });

    c.bench_function("codec/decode/large_payload", |b| {
        b.iter(|| {
            black_box(&recv_codec).decode(black_box(&encoded)).unwrap();
        });
    });
}

fn benchmark_codec_vec_operations(c: &mut Criterion) {
    let send_schema = parse_schema(
        r#"
        #[send]
        struct VecBench {
            data: Vec<u16> = [1, 2, 3, 4, 5],
        }
    "#,
    )
    .expect("Failed to parse send schema");

    let recv_schema = parse_schema(
        r#"
        #[receive]
        struct VecBench {
            data: Vec<u16>,
        }
    "#,
    )
    .expect("Failed to parse receive schema");

    let send_codec = Codec::compile(&send_schema, "VecBench").unwrap();
    let encoded = send_codec.encode().unwrap();

    let recv_codec = Codec::compile(&recv_schema, "VecBench").unwrap();

    c.bench_function("codec/encode/vec", |b| {
        b.iter(|| {
            black_box(&send_codec).encode().unwrap();
        });
    });

    c.bench_function("codec/decode/vec", |b| {
        b.iter(|| {
            black_box(&recv_codec).decode(black_box(&encoded)).unwrap();
        });
    });
}

fn benchmark_codec_string_operations(c: &mut Criterion) {
    let send_schema = parse_schema(
        r#"
        #[send]
        struct StringBench {
            short: String = "hi",
            medium: String = "This is a medium length string for testing",
            long: String = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.",
        }
    "#,
    )
    .expect("Failed to parse send schema");

    let recv_schema = parse_schema(
        r#"
        #[receive]
        struct StringBench {
            short: String,
            medium: String,
            long: String,
        }
    "#,
    )
    .expect("Failed to parse receive schema");

    let send_codec = Codec::compile(&send_schema, "StringBench").unwrap();
    let encoded = send_codec.encode().unwrap();

    let recv_codec = Codec::compile(&recv_schema, "StringBench").unwrap();

    c.bench_function("codec/encode/strings", |b| {
        b.iter(|| {
            black_box(&send_codec).encode().unwrap();
        });
    });

    c.bench_function("codec/decode/strings", |b| {
        b.iter(|| {
            black_box(&recv_codec).decode(black_box(&encoded)).unwrap();
        });
    });
}

fn benchmark_codec_endian_operations(c: &mut Criterion) {
    let send_schema = parse_schema(
        r#"
        #[endian(little)]
        #[send]
        struct EndianBench {
            a: u32 = 0x12345678,
            b: u64 = 0x0102030405060708,
            c: f32 = 3.14,
            d: f64 = 2.718281828,
        }
    "#,
    )
    .expect("Failed to parse send schema");

    let recv_schema = parse_schema(
        r#"
        #[endian(little)]
        #[receive]
        struct EndianBench {
            a: u32,
            b: u64,
            c: f32,
            d: f64,
        }
    "#,
    )
    .expect("Failed to parse receive schema");

    let send_codec = Codec::compile(&send_schema, "EndianBench").unwrap();
    let encoded = send_codec.encode().unwrap();

    let recv_codec = Codec::compile(&recv_schema, "EndianBench").unwrap();

    c.bench_function("codec/encode/little_endian", |b| {
        b.iter(|| {
            black_box(&send_codec).encode().unwrap();
        });
    });

    c.bench_function("codec/decode/little_endian", |b| {
        b.iter(|| {
            black_box(&recv_codec).decode(black_box(&encoded)).unwrap();
        });
    });
}

fn benchmark_codec_auto_fields(c: &mut Criterion) {
    // Auto fields require special syntax - using a simpler benchmark
    let send_schema = parse_schema(
        r#"
        #[send]
        struct AutoFieldBench {
            len: u32 = 1004,
            header: u32 = 0x12345678,
            data: Bytes = 1000,
            checksum: u32 = 0xDEADBEEF,
        }
    "#,
    )
    .expect("Failed to parse send schema");

    let recv_schema = parse_schema(
        r#"
        #[receive]
        struct AutoFieldBench {
            len: u32,
            header: u32,
            data: Bytes = 1000,
            checksum: u32,
        }
    "#,
    )
    .expect("Failed to parse receive schema");

    let send_codec = Codec::compile(&send_schema, "AutoFieldBench").unwrap();
    let encoded = send_codec.encode().unwrap();

    let recv_codec = Codec::compile(&recv_schema, "AutoFieldBench").unwrap();

    c.bench_function("codec/encode/with_bytes", |b| {
        b.iter(|| {
            black_box(&send_codec).encode().unwrap();
        });
    });

    c.bench_function("codec/decode/with_bytes", |b| {
        b.iter(|| {
            black_box(&recv_codec).decode(black_box(&encoded)).unwrap();
        });
    });
}

criterion_group!(
    benches,
    benchmark_codec_encode,
    benchmark_codec_decode,
    benchmark_codec_large_payload,
    benchmark_codec_vec_operations,
    benchmark_codec_string_operations,
    benchmark_codec_endian_operations,
    benchmark_codec_auto_fields,
);
criterion_main!(benches);
