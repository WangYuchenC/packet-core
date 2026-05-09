//! 解码性能基准测试

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use packet_core::{parse_schema, Codec};

const SMALL_SCHEMA: &str = r#"
#[send]
struct SmallPacket {
    id: u8 = 1,
    flag: bool = true,
}
#[receive]
struct SmallPacket {
    id: u8,
    flag: bool,
}
"#;

fn benchmark_decode_small(c: &mut Criterion) {
    let schema = parse_schema(SMALL_SCHEMA).unwrap();
    let codec = Codec::compile(&schema, "SmallPacket").unwrap();
    let encoded = codec.encode().unwrap();

    let mut group = c.benchmark_group("decode_small");
    group.throughput(Throughput::Bytes(encoded.len() as u64));
    group.bench_function("2_bytes", |b| {
        b.iter(|| black_box(&codec).decode(black_box(&encoded)).unwrap())
    });
    group.finish();
}

fn benchmark_decode_medium(c: &mut Criterion) {
    let schema = parse_schema(
        r#"
#[send]
struct MediumPacket {
    id: u32 = 0x12345678,
    temperature: f64 = 25.5,
    name: String = "test_sensor",
    values: Vec<u16>,
}
#[receive]
struct MediumPacket {
    id: u32,
    temperature: f64,
    name: String,
    values: Vec<u16>,
}
"#,
    )
    .unwrap();
    let codec = Codec::compile(&schema, "MediumPacket").unwrap();
    let encoded = codec.encode().unwrap();

    let mut group = c.benchmark_group("decode_medium");
    group.throughput(Throughput::Bytes(encoded.len() as u64));
    group.bench_function("medium_struct", |b| {
        b.iter(|| black_box(&codec).decode(black_box(&encoded)).unwrap())
    });
    group.finish();
}

fn benchmark_decode_large(c: &mut Criterion) {
    let schema = parse_schema(
        r#"
#[send]
struct LargePacket {
    header: u64 = 0xDEADBEEFCAFEBABE,
    data: Bytes = 50000,
    footer: u32 = 0x12345678,
}
#[receive]
struct LargePacket {
    header: u64,
    data: Bytes = 50000,
    footer: u32,
}
"#,
    )
    .unwrap();
    let codec = Codec::compile(&schema, "LargePacket").unwrap();
    let encoded = codec.encode().unwrap();

    let mut group = c.benchmark_group("decode_large");
    group.sample_size(10);
    group.throughput(Throughput::Bytes(encoded.len() as u64));
    group.bench_function("50KB", |b| {
        b.iter(|| black_box(&codec).decode(black_box(&encoded)).unwrap())
    });
    group.finish();
}

criterion_group!(
    benches,
    benchmark_decode_small,
    benchmark_decode_medium,
    benchmark_decode_large
);
criterion_main!(benches);
