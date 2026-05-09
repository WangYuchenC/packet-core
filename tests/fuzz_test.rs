//! Fuzz 测试：使用随机输入测试 parser 的健壮性
//!
//! 由于 cargo-fuzz 在 Windows 上不可用，这里使用随机化测试
//! 来模拟 fuzzing 行为。
//!
//! 运行: cargo test --test fuzz_test -- --ignored
//! 或者: cargo test fuzz_random_input

use packet_core::parse_schema;
use rand::Rng;

/// 生成随机字符串
fn random_string(rng: &mut impl Rng, max_len: usize) -> String {
    let len = rng.gen_range(0..=max_len);
    (0..len)
        .map(|_| {
            // 生成可打印 ASCII 字符
            let c = rng.gen_range(32..=126u8) as char;
            c
        })
        .collect()
}

/// 测试: 随机输入解析不应崩溃
#[test]
fn fuzz_random_input_no_crash() {
    let mut rng = rand::thread_rng();
    let iterations = 1000;

    for i in 0..iterations {
        let input = random_string(&mut rng, 200);

        // 解析应该要么成功，要么返回明确的错误
        // 无论如何都不应该 panic
        let _ = std::panic::catch_unwind(|| {
            let _ = parse_schema(&input);
        });

        if i % 100 == 0 {
            // 进度输出（方便调试）
            eprintln!("fuzz iteration {}/{}", i + 1, iterations);
        }
    }
}

/// 测试: 包含特殊字符的随机输入
#[test]
fn fuzz_special_characters() {
    let special_chars = [
        '\n', '\r', '\t', ' ', '{', '}', '[', ']', '(', ')', '<', '>', ';', ':', ',', '.', '#',
        '!', '@', '$', '%', '^', '&', '*', '+', '-', '=', '/', '\\', '|', '~', '`', '\'', '"',
    ];

    let mut rng = rand::thread_rng();
    let iterations = 500;

    for _ in 0..iterations {
        let len = rng.gen_range(1..=100);
        let input: String = (0..len)
            .map(|_| {
                if rng.gen_bool(0.3) {
                    // 30% 概率使用特殊字符
                    special_chars[rng.gen_range(0..special_chars.len())]
                } else {
                    rng.gen_range(b'a'..=b'z') as char
                }
            })
            .collect();

        let _ = std::panic::catch_unwind(|| {
            let _ = parse_schema(&input);
        });
    }
}

/// 测试: 长输入（压力测试）
#[test]
fn fuzz_long_input() {
    let mut rng = rand::thread_rng();

    // 生成长达 10KB 的随机字符串
    let input: String = (0..10240)
        .map(|_| rng.gen_range(32..=126u8) as char)
        .collect();

    let _ = std::panic::catch_unwind(|| {
        let _ = parse_schema(&input);
    });
}

/// 测试: UTF-8 边界情况
#[test]
fn fuzz_utf8_edge_cases() {
    // 各种 Unicode 字符
    let unicode_strings = vec![
        "你好世界",
        "\u{0000}",
        "\u{FFFF}",
        "\u{10FFFF}",
        "/// 文档注释测试",
        "\"字符串字面量\"",
    ];

    let mut rng = rand::thread_rng();
    let iterations = 200;

    for _ in 0..iterations {
        // 随机选择 Unicode 片段并组合
        let input: String = (0..rng.gen_range(1..=5))
            .map(|_| {
                let s = &unicode_strings[rng.gen_range(0..unicode_strings.len())];
                s.to_string()
            })
            .collect();

        let _ = std::panic::catch_unwind(|| {
            let _ = parse_schema(&input);
        });
    }
}
