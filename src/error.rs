//! Structured decode errors.

use core::fmt;

/// Error produced when decoding rencodeplus bytes fails.
///
/// Carries the [`DecodeErrorKind`] and the byte offset in the input at which
/// the decoder detected the problem. The decoder never panics on malformed
/// input; every failure is reported through this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    kind: DecodeErrorKind,
    offset: usize,
}

impl DecodeError {
    pub(crate) fn new(kind: DecodeErrorKind, offset: usize) -> Self {
        Self { kind, offset }
    }

    /// The kind of failure.
    pub fn kind(&self) -> &DecodeErrorKind {
        &self.kind
    }

    /// Byte offset in the input at which the decoder detected the problem.
    ///
    /// For truncation errors this is the start of the data region that was
    /// cut short; for content errors it is the start of the offending
    /// element.
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at offset {}", self.kind, self.offset)
    }
}

impl std::error::Error for DecodeError {}

/// The specific way an input failed to decode.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeErrorKind {
    /// The input was empty.
    EmptyInput,
    /// The input ended before the current record was complete.
    UnexpectedEnd,
    /// A byte that is not a valid type code appeared where a value was
    /// expected.
    UnknownTypeCode(u8),
    /// A terminator byte (`0x7f`) appeared where a value was required.
    UnexpectedTerminator,
    /// A text string was not valid UTF-8.
    InvalidUtf8,
    /// A decimal length prefix was too large to represent.
    InvalidLength,
    /// A decimal length prefix was not followed by `:` (text) or `/` (bytes).
    MissingLengthSeparator,
    /// A declared string or byte-string length exceeded the remaining input.
    LengthExceedsInput {
        /// The length declared by the decimal prefix.
        declared: u64,
        /// The number of bytes actually remaining in the input.
        available: usize,
    },
    /// A variable-length list or dictionary ended without its terminator.
    UnterminatedContainer,
    /// Big decimal integer text was 64 bytes or longer.
    BigIntTooLong,
    /// Big decimal integer text was not a valid signed decimal integer.
    InvalidBigInt,
    /// A decoded integer does not fit in the supported signed 64-bit range.
    IntegerOutOfRange,
    /// Strict decoding found bytes after the first complete top-level value.
    TrailingBytes {
        /// The number of unconsumed bytes after the value.
        count: usize,
    },
    /// Nesting exceeded the configured maximum depth.
    DepthLimitExceeded {
        /// The configured limit that was exceeded.
        max_depth: usize,
    },
    /// A declared length exceeded the configured maximum allocation size.
    AllocLimitExceeded {
        /// The length declared by the input.
        requested: u64,
        /// The configured limit that was exceeded.
        max_alloc: usize,
    },
}

impl DecodeErrorKind {
    /// Stable machine-readable token naming this kind.
    ///
    /// Intended for scripted black-box testing (the CLI prints it); the
    /// tokens are part of the crate's public contract and will not change
    /// meaning across patch releases.
    pub fn name(&self) -> &'static str {
        match self {
            DecodeErrorKind::EmptyInput => "empty-input",
            DecodeErrorKind::UnexpectedEnd => "unexpected-end",
            DecodeErrorKind::UnknownTypeCode(_) => "unknown-type-code",
            DecodeErrorKind::UnexpectedTerminator => "unexpected-terminator",
            DecodeErrorKind::InvalidUtf8 => "invalid-utf8",
            DecodeErrorKind::InvalidLength => "invalid-length",
            DecodeErrorKind::MissingLengthSeparator => "missing-length-separator",
            DecodeErrorKind::LengthExceedsInput { .. } => "length-exceeds-input",
            DecodeErrorKind::UnterminatedContainer => "unterminated-container",
            DecodeErrorKind::BigIntTooLong => "big-int-too-long",
            DecodeErrorKind::InvalidBigInt => "invalid-big-int",
            DecodeErrorKind::IntegerOutOfRange => "integer-out-of-range",
            DecodeErrorKind::TrailingBytes { .. } => "trailing-bytes",
            DecodeErrorKind::DepthLimitExceeded { .. } => "depth-limit-exceeded",
            DecodeErrorKind::AllocLimitExceeded { .. } => "alloc-limit-exceeded",
        }
    }
}

impl fmt::Display for DecodeErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeErrorKind::EmptyInput => f.write_str("empty input"),
            DecodeErrorKind::UnexpectedEnd => f.write_str("unexpected end of input"),
            DecodeErrorKind::UnknownTypeCode(code) => {
                write!(f, "unknown type code 0x{code:02x}")
            }
            DecodeErrorKind::UnexpectedTerminator => {
                f.write_str("terminator byte 0x7f where a value was expected")
            }
            DecodeErrorKind::InvalidUtf8 => f.write_str("text string is not valid UTF-8"),
            DecodeErrorKind::InvalidLength => {
                f.write_str("decimal length prefix is too large to represent")
            }
            DecodeErrorKind::MissingLengthSeparator => {
                f.write_str("decimal length prefix is not followed by ':' or '/'")
            }
            DecodeErrorKind::LengthExceedsInput {
                declared,
                available,
            } => write!(
                f,
                "declared length {declared} exceeds the {available} remaining input byte(s)"
            ),
            DecodeErrorKind::UnterminatedContainer => {
                f.write_str("variable-length container is missing its terminator")
            }
            DecodeErrorKind::BigIntTooLong => {
                f.write_str("big decimal integer text is 64 bytes or longer")
            }
            DecodeErrorKind::InvalidBigInt => {
                f.write_str("big decimal integer text is not a valid signed decimal integer")
            }
            DecodeErrorKind::IntegerOutOfRange => {
                f.write_str("integer does not fit in the supported signed 64-bit range")
            }
            DecodeErrorKind::TrailingBytes { count } => {
                write!(f, "{count} trailing byte(s) after the top-level value")
            }
            DecodeErrorKind::DepthLimitExceeded { max_depth } => {
                write!(
                    f,
                    "nesting depth exceeds the configured limit of {max_depth}"
                )
            }
            DecodeErrorKind::AllocLimitExceeded {
                requested,
                max_alloc,
            } => write!(
                f,
                "declared length {requested} exceeds the configured allocation limit of {max_alloc}"
            ),
        }
    }
}
