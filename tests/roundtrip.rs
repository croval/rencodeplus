//! Round-trip and property tests: values survive encode → decode unchanged,
//! text and bytes never trade places, order is preserved, and encoding is
//! deterministic and canonical.

mod common;

use rencodeplus::{Map, Value, decode, decode_first, encode};

fn roundtrip(value: &Value) -> Value {
    decode(&encode(value)).expect("encoded value must decode")
}

#[test]
fn text_and_bytes_stay_distinct() {
    let text = Value::from("hello");
    let bytes = Value::from(b"hello".as_slice());
    assert_ne!(encode(&text), encode(&bytes));
    assert_eq!(roundtrip(&text), text);
    assert_eq!(roundtrip(&bytes), bytes);
    assert!(roundtrip(&text).as_text().is_some());
    assert!(roundtrip(&bytes).as_bytes().is_some());

    // Same distinction inside containers, including non-UTF-8 bytes.
    let mixed = Value::List(vec![
        Value::from(""),
        Value::Bytes(vec![]),
        Value::from("é"),
        Value::from(b"\xc3\x28"), // invalid UTF-8, must stay bytes
    ]);
    assert_eq!(roundtrip(&mixed), mixed);
}

#[test]
fn list_order_is_preserved() {
    let list = Value::List((0..100).map(Value::from).collect());
    assert_eq!(roundtrip(&list), list);
}

#[test]
fn map_insertion_order_is_preserved_on_the_wire() {
    let mut ab = Map::new();
    ab.push("a", 1);
    ab.push("b", 2);
    let mut ba = Map::new();
    ba.push("b", 2);
    ba.push("a", 1);
    // Different insertion orders produce different (both valid) bytes.
    assert_ne!(
        encode(&Value::Map(ab.clone())),
        encode(&Value::Map(ba.clone()))
    );
    assert_eq!(roundtrip(&Value::Map(ab.clone())), Value::Map(ab));
    assert_eq!(roundtrip(&Value::Map(ba.clone())), Value::Map(ba));
}

#[test]
fn duplicate_map_keys_are_preserved_and_lookup_takes_the_last() {
    // 67+2 = 0x68: fixed dictionary with two entries using the same key.
    let input = common::hex("68 81 61 01 81 61 02");
    let decoded = decode(&input).unwrap();
    let map = decoded.as_map().unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get_text("a"), Some(&Value::Int(2)));
    assert_eq!(map.get(&Value::from("a")), Some(&Value::Int(2)));
    // Re-encoding preserves both entries in wire order.
    assert_eq!(encode(&decoded), input);
}

#[test]
fn non_text_map_keys_roundtrip() {
    let mut map = Map::new();
    map.push(1, "one");
    map.push(Value::Bytes(vec![0x00]), "zero-byte");
    map.push(Value::Null, Value::Null);
    let value = Value::Map(map);
    assert_eq!(roundtrip(&value), value);
}

#[test]
fn float_specials_roundtrip() {
    for x in [
        0.0f64,
        1.5,
        -1.5,
        1e300,
        -1e-300,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        let decoded = roundtrip(&Value::Float(x));
        assert_eq!(decoded.as_float().unwrap().to_bits(), x.to_bits());
    }
    // NaN: semantic check, plus payload bits happen to be preserved by the
    // binary64 form.
    let decoded = roundtrip(&Value::Float(f64::NAN));
    assert!(decoded.as_float().unwrap().is_nan());
}

#[test]
fn integer_sweep_roundtrips_across_all_widths() {
    let mut cases: Vec<i64> = Vec::new();
    for boundary in [
        0i64, 1, 43, 44, 127, 128, 32767, 32768, 2147483647, 2147483648,
    ] {
        cases.extend([boundary - 1, boundary, boundary + 1]);
        cases.extend([-boundary - 1, -boundary, -boundary + 1]);
    }
    cases.extend([i64::MAX, i64::MIN, i64::MAX - 1, i64::MIN + 1, -32, -33]);
    for n in cases {
        assert_eq!(roundtrip(&Value::Int(n)), Value::Int(n), "n={n}");
    }
}

#[test]
fn realistic_nested_packet_roundtrips() {
    let mut screen = Map::new();
    screen.push("width", 1920);
    screen.push("height", 1080);
    let mut caps = Map::new();
    caps.push("version", "6.5.1");
    caps.push("rencodeplus", true);
    caps.push("session-id", Value::from(b"\x01\x02\x03\x04\xfe\xff"));
    caps.push("screen", Value::Map(screen));
    caps.push(
        "encodings",
        Value::List(vec![Value::from("rgb32"), Value::from("png")]),
    );
    caps.push("compression", Value::Null);
    caps.push("batch.delay", 15);
    let packet = Value::List(vec![Value::from("hello"), Value::Map(caps)]);
    assert_eq!(roundtrip(&packet), packet);
}

#[test]
fn encoding_is_deterministic_and_canonical() {
    let mut map = Map::new();
    map.push("k", Value::List(vec![Value::from(-42), Value::from("v")]));
    let value = Value::Map(map);
    let first = encode(&value);
    let second = encode(&value);
    assert_eq!(first, second);
    // Canonical fixed point: re-encoding a decoded value reproduces the
    // same bytes.
    assert_eq!(encode(&decode(&first).unwrap()), first);
}

#[test]
fn large_flat_structures_decode_under_default_limits() {
    let list = Value::List((0..100_000).map(Value::from).collect());
    let bytes = encode(&list);
    assert_eq!(decode(&bytes).unwrap(), list);

    let blob = Value::Bytes(vec![0xa5; 1_000_000]);
    let bytes = encode(&blob);
    assert_eq!(decode(&bytes).unwrap(), blob);
}

#[test]
fn pseudorandom_values_roundtrip() {
    // Deterministic value generator driven by xorshift64.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
    }
    fn gen_value(rng: &mut Rng, depth: usize) -> Value {
        let pick = if depth >= 4 {
            rng.next() % 6
        } else {
            rng.next() % 8
        };
        match pick {
            0 => Value::Null,
            1 => Value::Bool(rng.next().is_multiple_of(2)),
            2 => Value::Int(rng.next() as i64),
            3 => Value::Float(f64::from_bits(rng.next() & !0x7ff0_0000_0000_0000)),
            4 => {
                let len = (rng.next() % 80) as usize;
                Value::Text("xé".chars().cycle().take(len).collect())
            }
            5 => {
                let len = (rng.next() % 80) as usize;
                Value::Bytes((0..len).map(|_| rng.next() as u8).collect())
            }
            6 => {
                let len = (rng.next() % 70) as usize;
                Value::List((0..len).map(|_| gen_value(rng, depth + 1)).collect())
            }
            _ => {
                let len = (rng.next() % 30) as usize;
                let mut map = Map::new();
                for _ in 0..len {
                    map.push(gen_value(rng, depth + 1), gen_value(rng, depth + 1));
                }
                Value::Map(map)
            }
        }
    }

    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..300 {
        let value = gen_value(&mut rng, 0);
        let bytes = encode(&value);
        assert_eq!(decode(&bytes).unwrap(), value);
        // Lenient decode consumes exactly the whole encoding.
        let (lenient, consumed) = decode_first(&bytes).unwrap();
        assert_eq!(lenient, value);
        assert_eq!(consumed, bytes.len());
    }
}
