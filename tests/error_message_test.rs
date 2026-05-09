//! Test to verify error message propagation
//!
//! Run with: cargo test --test error_message_test -- --nocapture

use packet_core::parse_schema;

#[test]
fn test_error_message_contains_details() {
    let source = "strcut Header { magic: u8 }";
    let result = parse_schema(source);

    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_display = format!("{}", err);

    println!("Error message: {}", err_display);
    assert!(
        err_display.contains("syntax error") || err_display.contains("parse error"),
        "Error message should contain details, got: {}",
        err_display
    );
}

#[test]
fn test_error_message_for_invalid_token() {
    let source = "struct @invalid";
    let result = parse_schema(source);

    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_display = format!("{}", err);

    assert!(
        err_display.contains("syntax error")
            || err_display.contains("parse error")
            || err_display.contains("invalid"),
        "Error message should describe the invalid token, got: {}",
        err_display
    );
}
