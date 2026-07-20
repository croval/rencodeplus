//! Canonical encoder.
//!
//! The encoder always emits the canonical shortest wire form for every
//! value, as required for byte-for-byte compatibility: fixed single-byte
//! integers where possible, then progressively wider signed big-endian
//! forms; fixed-form strings, lists and dictionaries below the size
//! thresholds and variable-length forms above them; decimal length text
//! without leading zeroes.

use crate::value::Value;

/// Encodes `value` into a new byte vector.
///
/// Encoding is infallible for every representable [`Value`]: integers are
/// bounded to `i64` (which always fits an 8-byte record) and container
/// traversal uses an explicit work stack, so arbitrarily deep values cannot
/// overflow the call stack.
pub fn encode(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode_into(value, &mut out);
    out
}

/// Appends the canonical encoding of `value` to `out`.
pub fn encode_into(value: &Value, out: &mut Vec<u8>) {
    enum Job<'a> {
        Value(&'a Value),
        Literal(u8),
    }

    let mut stack = vec![Job::Value(value)];
    while let Some(job) = stack.pop() {
        let value = match job {
            Job::Literal(byte) => {
                out.push(byte);
                continue;
            }
            Job::Value(value) => value,
        };
        match value {
            Value::Null => out.push(0x45),
            Value::Bool(true) => out.push(0x43),
            Value::Bool(false) => out.push(0x44),
            Value::Int(n) => encode_int(*n, out),
            Value::Float(x) => {
                out.push(0x2c);
                out.extend_from_slice(&x.to_be_bytes());
            }
            Value::Text(s) => {
                let bytes = s.as_bytes();
                if bytes.len() < 64 {
                    out.push(0x80 + bytes.len() as u8);
                } else {
                    push_decimal(bytes.len(), out);
                    out.push(0x3a);
                }
                out.extend_from_slice(bytes);
            }
            Value::Bytes(bytes) => {
                push_decimal(bytes.len(), out);
                out.push(0x2f);
                out.extend_from_slice(bytes);
            }
            Value::List(items) => {
                if items.len() <= 63 {
                    out.push(0xc0 + items.len() as u8);
                } else {
                    out.push(0x3b);
                    stack.push(Job::Literal(0x7f));
                }
                for item in items.iter().rev() {
                    stack.push(Job::Value(item));
                }
            }
            Value::Map(map) => {
                if map.len() <= 24 {
                    out.push(0x66 + map.len() as u8);
                } else {
                    out.push(0x3c);
                    stack.push(Job::Literal(0x7f));
                }
                for (key, value) in map.entries().iter().rev() {
                    stack.push(Job::Value(value));
                    stack.push(Job::Value(key));
                }
            }
        }
    }
}

fn encode_int(n: i64, out: &mut Vec<u8>) {
    match n {
        0..=43 => out.push(n as u8),
        -32..=-1 => out.push((69 - n) as u8),
        -128..=127 => {
            out.push(0x3e);
            out.extend_from_slice(&(n as i8).to_be_bytes());
        }
        -32768..=32767 => {
            out.push(0x3f);
            out.extend_from_slice(&(n as i16).to_be_bytes());
        }
        -2147483648..=2147483647 => {
            out.push(0x40);
            out.extend_from_slice(&(n as i32).to_be_bytes());
        }
        _ => {
            out.push(0x41);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }
}

/// Appends the canonical decimal text for `n`: ASCII digits, no leading
/// zeroes except the single digit `0`.
fn push_decimal(mut n: usize, out: &mut Vec<u8>) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    out.extend_from_slice(&buf[i..]);
}
