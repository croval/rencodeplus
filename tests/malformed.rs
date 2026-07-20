//! Every malformed-input case listed in the specification text must return a
//! structured error — never panic — plus safety-limit behavior and no-panic
//! sweeps over adversarial inputs.

mod common;

use common::hex;
use rencodeplus::{DecodeConfig, DecodeErrorKind, Value, decode, decode_first, decode_with_config};

fn kind_of(input: &[u8]) -> DecodeErrorKind {
    decode(input)
        .expect_err("input must be rejected")
        .kind()
        .clone()
}

#[test]
fn empty_input() {
    assert_eq!(kind_of(&[]), DecodeErrorKind::EmptyInput);
}

#[test]
fn unknown_type_codes() {
    for code in [0x2du8, 0x2e, 0x2f, 0x3a] {
        assert_eq!(kind_of(&[code]), DecodeErrorKind::UnknownTypeCode(code));
    }
}

#[test]
fn top_level_terminator() {
    assert_eq!(kind_of(&[0x7f]), DecodeErrorKind::UnexpectedTerminator);
}

#[test]
fn truncated_fixed_width_records() {
    // Integers cut short.
    assert_eq!(kind_of(&hex("3e")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(kind_of(&hex("3f 00")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(kind_of(&hex("40 00 00 00")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(
        kind_of(&hex("41 00 00 00 00 00 00 00")),
        DecodeErrorKind::UnexpectedEnd
    );
    // Floats cut short.
    assert_eq!(
        kind_of(&hex("2c 3f f0 00 00 00 00 00")),
        DecodeErrorKind::UnexpectedEnd
    );
    assert_eq!(kind_of(&hex("42 3f 80 00")), DecodeErrorKind::UnexpectedEnd);
    // Big integer cut short before its terminator.
    assert_eq!(kind_of(&hex("3d")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(kind_of(&hex("3d 31 32")), DecodeErrorKind::UnexpectedEnd);
}

#[test]
fn truncated_strings_and_bytes() {
    // Fixed text form declaring more bytes than remain.
    assert_eq!(kind_of(&hex("81")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(kind_of(&hex("85 68 65")), DecodeErrorKind::UnexpectedEnd);
    // Long text and byte forms declaring more than remains.
    assert_eq!(
        kind_of(&hex("39 3a 61")),
        DecodeErrorKind::LengthExceedsInput {
            declared: 9,
            available: 1
        }
    );
    assert_eq!(
        kind_of(&hex("39 2f")),
        DecodeErrorKind::LengthExceedsInput {
            declared: 9,
            available: 0
        }
    );
    let mut long_text = hex("36 34 3a");
    long_text.extend(vec![0x61; 63]); // declares 64, provides 63
    assert_eq!(
        kind_of(&long_text),
        DecodeErrorKind::LengthExceedsInput {
            declared: 64,
            available: 63
        }
    );
}

#[test]
fn truncated_containers() {
    // Fixed lists with missing items.
    assert_eq!(kind_of(&hex("c1")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(kind_of(&hex("c2 01")), DecodeErrorKind::UnexpectedEnd);
    // Fixed dictionaries with missing keys or values.
    assert_eq!(kind_of(&hex("67")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(kind_of(&hex("67 81 61")), DecodeErrorKind::UnexpectedEnd);
    assert_eq!(kind_of(&hex("68 81 61 01")), DecodeErrorKind::UnexpectedEnd);
}

#[test]
fn invalid_utf8_in_text() {
    assert_eq!(kind_of(&hex("81 ff")), DecodeErrorKind::InvalidUtf8);
    assert_eq!(kind_of(&hex("82 c3 28")), DecodeErrorKind::InvalidUtf8);
    // Long text form with invalid UTF-8.
    assert_eq!(kind_of(&hex("31 3a ff")), DecodeErrorKind::InvalidUtf8);
    // Truncated multi-byte scalar at the end of a fixed string.
    assert_eq!(kind_of(&hex("81 c3")), DecodeErrorKind::InvalidUtf8);
    // The same bytes are fine as a byte string.
    assert_eq!(decode(&hex("31 2f ff")).unwrap(), Value::Bytes(vec![0xff]));
}

#[test]
fn length_prefix_without_separator() {
    // Digits then end of input.
    assert_eq!(kind_of(&hex("31")), DecodeErrorKind::MissingLengthSeparator);
    assert_eq!(
        kind_of(&hex("31 32 33")),
        DecodeErrorKind::MissingLengthSeparator
    );
    // Digits then a byte that is neither a digit nor ':' nor '/'.
    assert_eq!(
        kind_of(&hex("31 41 61")),
        DecodeErrorKind::MissingLengthSeparator
    );
    assert_eq!(
        kind_of(&hex("31 80")),
        DecodeErrorKind::MissingLengthSeparator
    );
}

#[test]
fn length_prefix_overflow() {
    // 2^64 in decimal ("18446744073709551616") does not fit in u64.
    let mut input: Vec<u8> = b"18446744073709551616".to_vec();
    input.push(b':');
    assert_eq!(kind_of(&input), DecodeErrorKind::InvalidLength);
    // Absurdly long digit runs must also fail cleanly.
    let mut input: Vec<u8> = b"9".repeat(100);
    input.push(b'/');
    assert_eq!(kind_of(&input), DecodeErrorKind::InvalidLength);
}

#[test]
fn unterminated_variable_containers() {
    assert_eq!(kind_of(&hex("3b")), DecodeErrorKind::UnterminatedContainer);
    assert_eq!(
        kind_of(&hex("3b 01 02")),
        DecodeErrorKind::UnterminatedContainer
    );
    assert_eq!(kind_of(&hex("3c")), DecodeErrorKind::UnterminatedContainer);
    assert_eq!(
        kind_of(&hex("3c 81 61 01")),
        DecodeErrorKind::UnterminatedContainer
    );
    // End of input where a dictionary value should start.
    assert_eq!(kind_of(&hex("3c 81 61")), DecodeErrorKind::UnexpectedEnd);
    // Terminator where a dictionary value should start.
    assert_eq!(
        kind_of(&hex("3c 81 61 7f")),
        DecodeErrorKind::UnexpectedTerminator
    );
}

#[test]
fn big_decimal_integer_text_errors() {
    // Empty text.
    assert_eq!(kind_of(&hex("3d 7f")), DecodeErrorKind::InvalidBigInt);
    // Lone minus sign.
    assert_eq!(kind_of(&hex("3d 2d 7f")), DecodeErrorKind::InvalidBigInt);
    // Leading plus sign is not allowed.
    assert_eq!(kind_of(&hex("3d 2b 35 7f")), DecodeErrorKind::InvalidBigInt);
    // Non-digit characters.
    assert_eq!(
        kind_of(&hex("3d 31 2e 35 7f")), // "1.5"
        DecodeErrorKind::InvalidBigInt
    );
    assert_eq!(
        kind_of(&hex("3d 20 35 7f")), // " 5"
        DecodeErrorKind::InvalidBigInt
    );
    assert_eq!(
        kind_of(&hex("3d 31 2d 7f")), // "1-"
        DecodeErrorKind::InvalidBigInt
    );
}

#[test]
fn big_decimal_integer_text_length_limit() {
    // 64 bytes of text must be rejected even with a terminator present.
    let mut input = vec![0x3d];
    input.extend(b"1".repeat(64));
    input.push(0x7f);
    assert_eq!(kind_of(&input), DecodeErrorKind::BigIntTooLong);
    // ... and also when the input ends right after the 64th text byte.
    let mut input = vec![0x3d];
    input.extend(b"1".repeat(64));
    assert_eq!(kind_of(&input), DecodeErrorKind::BigIntTooLong);
    // 63 bytes of text is accepted.
    let mut input = vec![0x3d];
    input.extend(b"0".repeat(62));
    input.push(b'7');
    input.push(0x7f);
    assert_eq!(decode(&input).unwrap(), Value::Int(7));
}

#[test]
fn big_decimal_integer_out_of_range() {
    let encode_bigint = |text: &str| {
        let mut input = vec![0x3d];
        input.extend(text.as_bytes());
        input.push(0x7f);
        input
    };
    assert_eq!(
        kind_of(&encode_bigint("9223372036854775808")), // i64::MAX + 1
        DecodeErrorKind::IntegerOutOfRange
    );
    assert_eq!(
        kind_of(&encode_bigint("-9223372036854775809")), // i64::MIN - 1
        DecodeErrorKind::IntegerOutOfRange
    );
    assert_eq!(
        kind_of(&encode_bigint("100000000000000000000000000000")),
        DecodeErrorKind::IntegerOutOfRange
    );
}

#[test]
fn strict_decode_rejects_trailing_bytes() {
    let err = decode(&hex("45 45")).unwrap_err();
    assert_eq!(*err.kind(), DecodeErrorKind::TrailingBytes { count: 1 });
    assert_eq!(err.offset(), 1);

    let err = decode(&hex("c1 01 00 00 00")).unwrap_err();
    assert_eq!(*err.kind(), DecodeErrorKind::TrailingBytes { count: 3 });
    assert_eq!(err.offset(), 2);
}

#[test]
fn lenient_decode_reports_consumed_bytes() {
    let (value, consumed) = decode_first(&hex("45 45")).unwrap();
    assert_eq!(value, Value::Null);
    assert_eq!(consumed, 1);

    let (value, consumed) = decode_first(&hex("c2 01 81 61 ff ff")).unwrap();
    assert_eq!(value, Value::List(vec![Value::Int(1), Value::from("a")]));
    assert_eq!(consumed, 4);
}

#[test]
fn depth_limit() {
    let config = DecodeConfig {
        max_depth: 3,
        ..DecodeConfig::default()
    };
    // Depths 1, 2, 3: fine.
    assert!(decode_with_config(&hex("c1 c1 45"), &config).is_ok());
    // Depth 4: rejected.
    assert_eq!(
        *decode_with_config(&hex("c1 c1 c1 45"), &config)
            .unwrap_err()
            .kind(),
        DecodeErrorKind::DepthLimitExceeded { max_depth: 3 }
    );

    // Default config: 100 nested lists exceed max_depth 64 ...
    let mut deep = vec![0xc1; 100];
    deep.push(0x45);
    assert_eq!(
        *decode(&deep).unwrap_err().kind(),
        DecodeErrorKind::DepthLimitExceeded { max_depth: 64 }
    );
    // ... while 60 decode fine.
    let mut ok = vec![0xc1; 60];
    ok.push(0x45);
    assert!(decode(&ok).is_ok());

    // Unbounded variable-length nesting must also hit the limit, not the
    // stack.
    let hostile = vec![0x3b; 100_000];
    assert_eq!(
        *decode(&hostile).unwrap_err().kind(),
        DecodeErrorKind::DepthLimitExceeded { max_depth: 64 }
    );
}

#[test]
fn alloc_limit() {
    let config = DecodeConfig {
        max_alloc: 4,
        ..DecodeConfig::default()
    };
    // 5 declared bytes with the data present: rejected by the limit.
    assert_eq!(
        *decode_with_config(&hex("35 2f 68 65 6c 6c 6f"), &config)
            .unwrap_err()
            .kind(),
        DecodeErrorKind::AllocLimitExceeded {
            requested: 5,
            max_alloc: 4
        }
    );
    // 4 declared bytes: accepted.
    assert!(decode_with_config(&hex("34 2f 68 65 6c 6c"), &config).is_ok());
    // Wire-format truncation is reported before the allocation limit.
    assert_eq!(
        *decode_with_config(&hex("39 2f 61"), &config)
            .unwrap_err()
            .kind(),
        DecodeErrorKind::LengthExceedsInput {
            declared: 9,
            available: 1
        }
    );
}

#[test]
fn error_offsets_point_at_the_problem() {
    // The unknown code is the third byte (offset 2) inside the list.
    let err = decode(&hex("c2 01 2d")).unwrap_err();
    assert_eq!(*err.kind(), DecodeErrorKind::UnknownTypeCode(0x2d));
    assert_eq!(err.offset(), 2);

    // Truncation is reported at the start of the record's cut-short payload.
    let err = decode(&hex("3f 00")).unwrap_err();
    assert_eq!(err.offset(), 1);
}

#[test]
fn no_panic_on_all_one_and_two_byte_inputs() {
    for a in 0..=255u8 {
        let _ = decode(&[a]);
        for b in 0..=255u8 {
            let _ = decode(&[a, b]);
        }
    }
}

#[test]
fn no_panic_on_pseudorandom_inputs() {
    // Deterministic xorshift64 stream; no external dependencies.
    let mut state = 0x243f_6a88_85a3_08d3u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..50_000 {
        let len = (next() % 96) as usize;
        let buf: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        let _ = decode(&buf); // must return Ok or Err, never panic
    }
}

#[test]
fn no_panic_on_mutated_valid_encodings() {
    // Take a realistic nested packet, flip each byte through several values,
    // and require a clean Ok/Err either way.
    let mut map = rencodeplus::Map::new();
    map.push("version", "6.5.1");
    map.push("caps", Value::List(vec![Value::from(1), Value::from("x")]));
    map.push("blob", Value::from(b"\x00\x01\x02\xff"));
    let packet = rencodeplus::encode(&Value::List(vec![Value::from("hello"), Value::Map(map)]));
    for i in 0..packet.len() {
        for delta in [1u8, 0x40, 0x80, 0xff] {
            let mut mutated = packet.clone();
            mutated[i] = mutated[i].wrapping_add(delta);
            let _ = decode(&mutated);
        }
    }
}
