//! Byte-for-byte checks of every canonical test vector in the
//! specification text: encoding must produce exactly the expected bytes, and
//! decoding the expected bytes must produce exactly the original value.

mod common;

use common::{hex, to_hex};
use rencodeplus::{DecodeErrorKind, Map, Value, decode, encode};

/// Asserts the canonical vector in both directions.
fn check(value: Value, expected_hex: &str) {
    let expected = hex(expected_hex);
    let encoded = encode(&value);
    assert_eq!(
        to_hex(&encoded),
        to_hex(&expected),
        "encode mismatch for {value}"
    );
    let decoded = decode(&expected).expect("canonical vector must decode");
    assert_eq!(decoded, value, "decode mismatch for {expected_hex:?}");
}

#[test]
fn scalars() {
    check(Value::Null, "45");
    check(Value::Bool(true), "43");
    check(Value::Bool(false), "44");
    check(Value::Int(0), "00");
    check(Value::Int(1), "01");
    check(Value::Int(43), "2b");
    check(Value::Int(44), "3e 2c");
    check(Value::Int(127), "3e 7f");
    check(Value::Int(128), "3f 00 80");
    check(Value::Int(32767), "3f 7f ff");
    check(Value::Int(32768), "40 00 00 80 00");
    check(Value::Int(2147483647), "40 7f ff ff ff");
    check(Value::Int(2147483648), "41 00 00 00 00 80 00 00 00");
    check(Value::Int(-1), "46");
    check(Value::Int(-2), "47");
    check(Value::Int(-32), "65");
    check(Value::Int(-33), "3e df");
    check(Value::Int(-128), "3e 80");
    check(Value::Int(-129), "3f ff 7f");
    check(Value::Int(-32768), "3f 80 00");
    check(Value::Int(-32769), "40 ff ff 7f ff");
    check(Value::Int(-2147483648), "40 80 00 00 00");
    check(Value::Int(-2147483649), "41 ff ff ff ff 7f ff ff ff");
    check(Value::Float(1.0), "2c 3f f0 00 00 00 00 00 00");
    check(Value::Float(-0.0), "2c 80 00 00 00 00 00 00 00");
}

#[test]
fn negative_zero_sign_survives_decode() {
    // Value equality treats -0.0 == 0.0, so check the bit pattern directly.
    let decoded = decode(&hex("2c 80 00 00 00 00 00 00 00")).unwrap();
    let float = decoded.as_float().expect("must decode to a float");
    assert_eq!(float.to_bits(), (-0.0f64).to_bits());
}

#[test]
fn i64_extremes_use_eight_byte_form() {
    check(Value::Int(i64::MAX), "41 7f ff ff ff ff ff ff ff");
    check(Value::Int(i64::MIN), "41 80 00 00 00 00 00 00 00");
}

#[test]
fn text() {
    check(Value::Text(String::new()), "80");
    check(Value::from("a"), "81 61");
    check(Value::from("hello"), "85 68 65 6c 6c 6f");
    check(Value::from("é"), "82 c3 a9");

    // 64 ASCII `a` bytes: long text form "64:" then the bytes.
    let long = "a".repeat(64);
    let expected = format!("36 34 3a {}", "61 ".repeat(64));
    check(Value::from(long.as_str()), &expected);

    // 63 bytes is the last fixed-form length.
    let fixed = "a".repeat(63);
    let expected = format!("bf {}", "61 ".repeat(63));
    check(Value::from(fixed.as_str()), &expected);
}

#[test]
fn bytes() {
    check(Value::Bytes(vec![]), "30 2f");
    check(Value::from(b"\x61"), "31 2f 61");
    check(Value::from(b"hello".as_slice()), "35 2f 68 65 6c 6c 6f");
    check(Value::from(b"\x00\xff"), "32 2f 00 ff");
}

#[test]
fn lists() {
    check(Value::List(vec![]), "c0");
    check(Value::List(vec![Value::Int(1)]), "c1 01");
    check(
        Value::List(vec![Value::Int(1), Value::from("a")]),
        "c2 01 81 61",
    );
    check(
        Value::List(vec![Value::Bool(true), Value::Bool(false), Value::Null]),
        "c3 43 44 45",
    );
    check(Value::List(vec![Value::List(vec![])]), "c1 c0");

    // 64 items switches to the variable-length form with terminator.
    let zeros = Value::List(vec![Value::Int(0); 64]);
    let expected = format!("3b {} 7f", "00 ".repeat(64));
    check(zeros, &expected);

    // 63 items is the last fixed-form count.
    let fixed = Value::List(vec![Value::Int(0); 63]);
    let expected = format!("ff {}", "00 ".repeat(63));
    check(fixed, &expected);
}

#[test]
fn dictionaries() {
    check(Value::Map(Map::new()), "66");

    let mut map = Map::new();
    map.push("a", 1);
    check(Value::Map(map), "67 81 61 01");

    let mut map = Map::new();
    map.push("a", 1);
    map.push("b", true);
    check(Value::Map(map), "68 81 61 01 81 62 43");

    let mut map = Map::new();
    map.push("bytes", Value::from(b"\x00\xff"));
    check(Value::Map(map), "67 85 62 79 74 65 73 32 2f 00 ff");
}

#[test]
fn dictionary_form_thresholds() {
    // 24 items is the last fixed-form count (type code 0x7e)...
    let mut map = Map::new();
    for i in 0..24 {
        map.push(i, i);
    }
    let encoded = encode(&Value::Map(map.clone()));
    assert_eq!(encoded[0], 0x7e);
    assert_eq!(decode(&encoded).unwrap(), Value::Map(map));

    // ...and 25 items switches to the variable-length form with terminator.
    let mut map = Map::new();
    for i in 0..25 {
        map.push(i, i);
    }
    let encoded = encode(&Value::Map(map.clone()));
    assert_eq!(encoded[0], 0x3c);
    assert_eq!(*encoded.last().unwrap(), 0x7f);
    assert_eq!(decode(&encoded).unwrap(), Value::Map(map));
}

#[test]
fn big_integer_form_is_recognized_and_rejected_as_out_of_range() {
    // i64::MAX + 1: the crate supports only signed 64-bit integers, so this
    // must fail with a clear out-of-range error (spec: decoder-recognition).
    let input = hex("3d 39 32 32 33 33 37 32 30 33 36 38 35 34 37 37 35 38 30 38 7f");
    let err = decode(&input).unwrap_err();
    assert_eq!(*err.kind(), DecodeErrorKind::IntegerOutOfRange);
}
