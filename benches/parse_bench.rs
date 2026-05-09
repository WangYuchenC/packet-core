//! 解析性能基准测试
//!
//! 目标: 解析速度 > 10 MB/s

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use packet_core::parse_schema;

/// 简单 schema
const SIMPLE_SCHEMA: &str = r#"
struct TestSend {
    id: u32,
    value: u8,
}
"#;

/// 中等复杂度 schema
const MEDIUM_SCHEMA: &str = r#"
struct Header {
    magic: u32,
    version: u8,
}

struct DataPoint {
    timestamp: u64,
    value: f32,
}

struct Message {
    header: Header,
    data: [DataPoint; 10],
}
"#;

/// 生成大 schema
fn generate_large_schema(size: usize) -> String {
    let mut schema = String::new();
    schema.push_str("struct Header {\n    magic: u32,\n}\n\n");

    for i in 0..size {
        schema.push_str(&format!(
            "struct Data{} {{\n    field1: u32,\n    field2: u64,\n    field3: f32,\n}}\n\n",
            i
        ));
    }

    schema.push_str("struct LargeMessage {\n    header: Header,\n");
    for i in 0..size {
        schema.push_str(&format!("    data{}: Data{},\n", i, i));
    }
    schema.push_str("}\n");

    schema
}

/// 基准测试: 解析简单 schema
fn benchmark_parse_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_simple");
    group.throughput(Throughput::Bytes(SIMPLE_SCHEMA.len() as u64));

    group.bench_function("simple_schema", |b| {
        b.iter(|| black_box(parse_schema(SIMPLE_SCHEMA).unwrap()))
    });

    group.finish();
}

/// 基准测试: 解析中等复杂度 schema
fn benchmark_parse_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_medium");
    group.throughput(Throughput::Bytes(MEDIUM_SCHEMA.len() as u64));

    group.bench_function("medium_schema", |b| {
        b.iter(|| black_box(parse_schema(MEDIUM_SCHEMA).unwrap()))
    });

    group.finish();
}

/// 基准测试: 解析大 schema
fn benchmark_parse_large(c: &mut Criterion) {
    let large_schema = generate_large_schema(100);
    let mut group = c.benchmark_group("parse_large");
    group.throughput(Throughput::Bytes(large_schema.len() as u64));
    group.sample_size(10);

    group.bench_function("large_schema_100_structs", |b| {
        b.iter(|| black_box(parse_schema(&large_schema).unwrap()))
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_parse_simple,
    benchmark_parse_medium,
    benchmark_parse_large
);
criterion_main!(benches);
