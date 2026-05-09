//! 编码性能基准测试

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use packet_core::{parse_schema, Codec};

const SMALL_SCHEMA: &str = r#"
#[send]
struct SmallPacket {
    id: u8 = 1,
    flag: bool = true,
}
"#;

fn benchmark_encode_small(c: &mut Criterion) {
    let schema = parse_schema(SMALL_SCHEMA).unwrap();
    let codec = Codec::compile(&schema, "SmallPacket").unwrap();

    let mut group = c.benchmark_group("encode_small");
    group.throughput(Throughput::Bytes(2));
    group.bench_function("2_bytes", |b| {
        b.iter(|| black_box(&codec).encode().unwrap())
    });
    group.finish();
}

fn benchmark_encode_medium(c: &mut Criterion) {
    let schema = parse_schema(
        r#"
#[send]
struct MediumPacket {
    id: u32 = 0x12345678,
    temperature: f64 = 25.5,
    name: String = "test_sensor",
    values: Vec<u16>,
}
"#,
    )
    .unwrap();
    let codec = Codec::compile(&schema, "MediumPacket").unwrap();

    let mut group = c.benchmark_group("encode_medium");
    group.bench_function("medium_struct", |b| {
        b.iter(|| black_box(&codec).encode().unwrap())
    });
    group.finish();
}

fn benchmark_encode_large(c: &mut Criterion) {
    let schema = parse_schema(
        r#"
#[send]
struct LargePacket {
    header: u64 = 0xDEADBEEFCAFEBABE,
    data: Bytes = 50000,
    footer: u32 = 0x12345678,
}
"#,
    )
    .unwrap();
    let codec = Codec::compile(&schema, "LargePacket").unwrap();

    let mut group = c.benchmark_group("encode_large");
    group.sample_size(10);
    group.throughput(Throughput::Bytes(50016));
    group.bench_function("50KB", |b| b.iter(|| black_box(&codec).encode().unwrap()));
    group.finish();
}

criterion_group!(
    benches,
    benchmark_encode_small,
    benchmark_encode_medium,
    benchmark_encode_large
);
criterion_main!(benches);
