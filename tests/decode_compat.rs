//! Decode-only compatibility: the decoder must accept well-formed
//! non-canonical encodings (which the encoder never emits) and produce the
//! same logical value as the canonical form.

mod common;

use common::hex;
use rencodeplus::{Value, decode};

fn check(input_hex: &str, expected: Value) {
    let decoded = decode(&hex(input_hex)).expect("well-formed input must decode");
    assert_eq!(decoded, expected, "decode mismatch for {input_hex:?}");
}

#[test]
fn spec_decode_only_vectors() {
    check("42 3f 80 00 00", Value::Float(1.0)); // binary32 float
    check("31 3a 61", Value::from("a")); // long text form, short string
    check("30 3a", Value::Text(String::new())); // long text form, empty
}

#[test]
fn wider_than_necessary_integer_forms() {
    check("3e 05", Value::Int(5));
    check("3f 00 05", Value::Int(5));
    check("40 00 00 00 05", Value::Int(5));
    check("41 00 00 00 00 00 00 00 05", Value::Int(5));
    check("3e ff", Value::Int(-1));
    check("3f ff ff", Value::Int(-1));
    check("40 ff ff ff ff", Value::Int(-1));
    check("41 ff ff ff ff ff ff ff ff", Value::Int(-1));
}

#[test]
fn variable_length_containers_below_thresholds() {
    check("3b 7f", Value::List(vec![]));
    check("3b 01 7f", Value::List(vec![Value::Int(1)]));
    check("3c 7f", Value::Map(rencodeplus::Map::new()));
    let mut map = rencodeplus::Map::new();
    map.push("a", 1);
    check("3c 81 61 01 7f", Value::Map(map));
}

#[test]
fn leading_zeroes_in_decimal_lengths() {
    check("30 31 3a 61", Value::from("a")); // "01:a"
    check("30 30 3a", Value::Text(String::new())); // "00:"
    check("30 32 2f 00 ff", Value::from(b"\x00\xff")); // "02/…"
    check("30 30 2f", Value::Bytes(vec![])); // "00/"
    check(
        "30 30 30 30 35 2f 68 65 6c 6c 6f",
        Value::from(b"hello".as_slice()),
    ); // "00005/hello"
}

#[test]
fn big_decimal_integers_within_i64_range() {
    check("3d 31 32 7f", Value::Int(12)); // "12"
    check("3d 2d 31 32 7f", Value::Int(-12)); // "-12"
    check("3d 30 7f", Value::Int(0)); // "0"
    check("3d 2d 30 7f", Value::Int(0)); // "-0" decodes to zero per spec
    check("3d 30 30 37 7f", Value::Int(7)); // "007" leading zeroes
    check("3d 2d 30 30 37 7f", Value::Int(-7)); // "-007"

    // i64 boundaries expressed in big decimal form.
    let max = format!(
        "3d {} 7f",
        i64::MAX
            .to_string()
            .bytes()
            .map(|b| format!("{b:02x} "))
            .collect::<String>()
    );
    check(&max, Value::Int(i64::MAX));
    let min = format!(
        "3d {} 7f",
        i64::MIN
            .to_string()
            .bytes()
            .map(|b| format!("{b:02x} "))
            .collect::<String>()
    );
    check(&min, Value::Int(i64::MIN));

    // 63 bytes of decimal text is the longest accepted form.
    let text = format!("{}7", "0".repeat(62));
    assert_eq!(text.len(), 63);
    let vector = format!(
        "3d {} 7f",
        text.bytes()
            .map(|b| format!("{b:02x} "))
            .collect::<String>()
    );
    check(&vector, Value::Int(7));
}

#[test]
fn binary32_special_values() {
    check("42 00 00 00 00", Value::Float(0.0));
    check("42 c0 00 00 00", Value::Float(-2.0));
    // binary32 infinity widens to f64 infinity.
    check("42 7f 80 00 00", Value::Float(f64::INFINITY));
    // binary32 NaN widens to an f64 NaN (semantic check, not bit equality).
    let decoded = decode(&hex("42 7f c0 00 00")).unwrap();
    assert!(decoded.as_float().unwrap().is_nan());
}
