//! Strict-by-default decoder.
//!
//! [`decode`] is the recommended entry point for packet payloads: it decodes
//! exactly one top-level value and rejects trailing bytes. [`decode_first`]
//! is the opt-in lenient variant that accepts the first value and reports how
//! many bytes it consumed, leaving any trailing bytes to the caller.
//!
//! The decoder accepts well-formed non-canonical encodings (wider integer
//! forms, long text form for short strings, variable-length containers below
//! the fixed-form thresholds, leading zeroes in decimal text); the resulting
//! value is identical to the one the canonical form would produce.

use crate::error::{DecodeError, DecodeErrorKind};
use crate::value::{Map, Value};

/// Safety limits applied while decoding untrusted input.
///
/// These are API safety controls, not wire-format features: inputs rejected
/// by a limit may be well-formed rencodeplus that a more permissive
/// configuration would accept.
#[derive(Debug, Clone)]
pub struct DecodeConfig {
    /// Maximum nesting depth of the decoded value tree.
    ///
    /// A top-level scalar has depth 1 and each container level adds 1, so
    /// the limit must be at least 1 to decode anything. This also bounds the
    /// decoder's recursion, keeping stack usage proportional to the limit.
    pub max_depth: usize,
    /// Maximum accepted declared length, in bytes, for a single text or
    /// byte-string record.
    ///
    /// Declared lengths are validated against the remaining input before
    /// anything is allocated, so decoder memory use is always bounded by the
    /// input size; this limit adds an explicit ceiling below that.
    pub max_alloc: usize,
}

impl Default for DecodeConfig {
    /// Defaults suitable for untrusted network input carrying ordinary Xpra
    /// packets: `max_depth` 64, `max_alloc` 256 MiB.
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_alloc: 256 * 1024 * 1024,
        }
    }
}

/// Strictly decodes `input` as exactly one value with default limits.
///
/// Errors if the input is empty, malformed, or contains trailing bytes after
/// the first top-level value.
pub fn decode(input: &[u8]) -> Result<Value, DecodeError> {
    decode_with_config(input, &DecodeConfig::default())
}

/// Strictly decodes `input` as exactly one value with the given limits.
pub fn decode_with_config(input: &[u8], config: &DecodeConfig) -> Result<Value, DecodeError> {
    let (value, consumed) = decode_first_with_config(input, config)?;
    if consumed < input.len() {
        return Err(DecodeError::new(
            DecodeErrorKind::TrailingBytes {
                count: input.len() - consumed,
            },
            consumed,
        ));
    }
    Ok(value)
}

/// Leniently decodes the first value in `input` with default limits.
///
/// Returns the value and the number of bytes it consumed; trailing bytes are
/// ignored. This mirrors Xpra's own load behavior and exists for
/// compatibility testing — prefer [`decode`] at packet boundaries.
pub fn decode_first(input: &[u8]) -> Result<(Value, usize), DecodeError> {
    decode_first_with_config(input, &DecodeConfig::default())
}

/// Leniently decodes the first value in `input` with the given limits.
pub fn decode_first_with_config(
    input: &[u8],
    config: &DecodeConfig,
) -> Result<(Value, usize), DecodeError> {
    if input.is_empty() {
        return Err(DecodeError::new(DecodeErrorKind::EmptyInput, 0));
    }
    let mut decoder = Decoder {
        input,
        pos: 0,
        config,
    };
    let value = decoder.decode_value(1)?;
    Ok((value, decoder.pos))
}

struct Decoder<'a> {
    input: &'a [u8],
    pos: usize,
    config: &'a DecodeConfig,
}

impl<'a> Decoder<'a> {
    fn fail<T>(&self, kind: DecodeErrorKind, offset: usize) -> Result<T, DecodeError> {
        Err(DecodeError::new(kind, offset))
    }

    fn remaining(&self) -> usize {
        self.input.len() - self.pos
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next_byte(&mut self) -> Result<u8, DecodeError> {
        match self.input.get(self.pos) {
            Some(&byte) => {
                self.pos += 1;
                Ok(byte)
            }
            None => self.fail(DecodeErrorKind::UnexpectedEnd, self.pos),
        }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if n > self.remaining() {
            return self.fail(DecodeErrorKind::UnexpectedEnd, self.pos);
        }
        let slice = &self.input[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let slice = self.take(N)?;
        let mut array = [0u8; N];
        array.copy_from_slice(slice);
        Ok(array)
    }

    fn decode_value(&mut self, depth: usize) -> Result<Value, DecodeError> {
        if depth > self.config.max_depth {
            return self.fail(
                DecodeErrorKind::DepthLimitExceeded {
                    max_depth: self.config.max_depth,
                },
                self.pos,
            );
        }
        let start = self.pos;
        let code = self.next_byte()?;
        match code {
            // Fixed positive integers: the code is the value.
            0x00..=0x2b => Ok(Value::Int(i64::from(code))),
            // 64-bit float, eight big-endian IEEE-754 binary64 bytes.
            0x2c => Ok(Value::Float(f64::from_be_bytes(self.take_array()?))),
            // ASCII digit: decimal length prefix of a text (":") or byte
            // ("/") string.
            0x30..=0x39 => self.decode_length_prefixed(code),
            // Variable-length list, items until terminator.
            0x3b => {
                let mut items = Vec::new();
                loop {
                    match self.peek() {
                        None => return self.fail(DecodeErrorKind::UnterminatedContainer, self.pos),
                        Some(0x7f) => {
                            self.pos += 1;
                            break;
                        }
                        Some(_) => items.push(self.decode_value(depth + 1)?),
                    }
                }
                Ok(Value::List(items))
            }
            // Variable-length dictionary, key/value pairs until terminator.
            0x3c => {
                let mut map = Map::new();
                loop {
                    match self.peek() {
                        None => return self.fail(DecodeErrorKind::UnterminatedContainer, self.pos),
                        Some(0x7f) => {
                            self.pos += 1;
                            break;
                        }
                        Some(_) => {
                            let key = self.decode_value(depth + 1)?;
                            let value = self.decode_value(depth + 1)?;
                            map.push(key, value);
                        }
                    }
                }
                Ok(Value::Map(map))
            }
            // Big decimal integer, ASCII text until terminator.
            0x3d => self.decode_big_int(),
            // Fixed-width signed big-endian integers.
            0x3e => Ok(Value::Int(i64::from(self.take_array::<1>()?[0] as i8))),
            0x3f => Ok(Value::Int(i64::from(i16::from_be_bytes(
                self.take_array()?,
            )))),
            0x40 => Ok(Value::Int(i64::from(i32::from_be_bytes(
                self.take_array()?,
            )))),
            0x41 => Ok(Value::Int(i64::from_be_bytes(self.take_array()?))),
            // 32-bit float (decode-only), widened losslessly to f64.
            0x42 => Ok(Value::Float(f64::from(f32::from_be_bytes(
                self.take_array()?,
            )))),
            0x43 => Ok(Value::Bool(true)),
            0x44 => Ok(Value::Bool(false)),
            0x45 => Ok(Value::Null),
            // Fixed negative integers: 0x46 is -1 through 0x65 is -32.
            0x46..=0x65 => Ok(Value::Int(69 - i64::from(code))),
            // Fixed dictionaries: item count is code - 0x66.
            0x66..=0x7e => {
                let count = usize::from(code - 0x66);
                let mut map = Map::with_capacity(count);
                for _ in 0..count {
                    let key = self.decode_value(depth + 1)?;
                    let value = self.decode_value(depth + 1)?;
                    map.push(key, value);
                }
                Ok(Value::Map(map))
            }
            0x7f => self.fail(DecodeErrorKind::UnexpectedTerminator, start),
            // Fixed UTF-8 strings: byte length is code - 0x80.
            0x80..=0xbf => {
                let len = usize::from(code - 0x80);
                let text_start = self.pos;
                let bytes = self.take(len)?;
                match core::str::from_utf8(bytes) {
                    Ok(s) => Ok(Value::Text(s.to_owned())),
                    Err(_) => self.fail(DecodeErrorKind::InvalidUtf8, text_start),
                }
            }
            // Fixed lists: item count is code - 0xc0.
            0xc0..=0xff => {
                let count = usize::from(code - 0xc0);
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(self.decode_value(depth + 1)?);
                }
                Ok(Value::List(items))
            }
            // Remaining codes: 0x2d, 0x2e, 0x2f, 0x3a.
            _ => self.fail(DecodeErrorKind::UnknownTypeCode(code), start),
        }
    }

    fn decode_length_prefixed(&mut self, first_digit: u8) -> Result<Value, DecodeError> {
        let prefix_start = self.pos - 1;
        let mut length = u64::from(first_digit - b'0');
        let separator = loop {
            let byte_pos = self.pos;
            let byte = match self.next_byte() {
                Ok(byte) => byte,
                Err(_) => {
                    return self.fail(DecodeErrorKind::MissingLengthSeparator, byte_pos);
                }
            };
            match byte {
                b'0'..=b'9' => {
                    length = match length
                        .checked_mul(10)
                        .and_then(|v| v.checked_add(u64::from(byte - b'0')))
                    {
                        Some(v) => v,
                        None => return self.fail(DecodeErrorKind::InvalidLength, prefix_start),
                    };
                }
                0x3a | 0x2f => break byte,
                _ => return self.fail(DecodeErrorKind::MissingLengthSeparator, byte_pos),
            }
        };
        let payload_start = self.pos;
        if length > self.remaining() as u64 {
            return self.fail(
                DecodeErrorKind::LengthExceedsInput {
                    declared: length,
                    available: self.remaining(),
                },
                payload_start,
            );
        }
        if length > self.config.max_alloc as u64 {
            return self.fail(
                DecodeErrorKind::AllocLimitExceeded {
                    requested: length,
                    max_alloc: self.config.max_alloc,
                },
                payload_start,
            );
        }
        // Cannot truncate: length <= remaining() <= usize::MAX.
        let bytes = self.take(length as usize)?;
        if separator == 0x3a {
            match core::str::from_utf8(bytes) {
                Ok(s) => Ok(Value::Text(s.to_owned())),
                Err(_) => self.fail(DecodeErrorKind::InvalidUtf8, payload_start),
            }
        } else {
            Ok(Value::Bytes(bytes.to_vec()))
        }
    }

    fn decode_big_int(&mut self) -> Result<Value, DecodeError> {
        let text_start = self.pos;
        loop {
            let byte = self.next_byte()?;
            if byte == 0x7f {
                break;
            }
            if self.pos - text_start >= 64 {
                return self.fail(DecodeErrorKind::BigIntTooLong, text_start);
            }
        }
        let text = &self.input[text_start..self.pos - 1];
        let digits = match text {
            [b'-', rest @ ..] => rest,
            _ => text,
        };
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            return self.fail(DecodeErrorKind::InvalidBigInt, text_start);
        }
        // The charset was validated above, so the text is ASCII and the only
        // way parsing can fail is a value outside the i64 range.
        let Ok(text) = core::str::from_utf8(text) else {
            return self.fail(DecodeErrorKind::InvalidBigInt, text_start);
        };
        match text.parse::<i64>() {
            Ok(n) => Ok(Value::Int(n)),
            Err(_) => self.fail(DecodeErrorKind::IntegerOutOfRange, text_start),
        }
    }
}
