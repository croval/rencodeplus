//! Clean-room encoder/decoder for the Xpra `rencodeplus` payload format.
//!
//! This crate serializes and deserializes the value payloads carried inside
//! Xpra protocol packets. It deliberately knows nothing about Xpra packet
//! framing, compression, chunking, transport, or packet names; those belong
//! to the protocol layer that wraps this crate.
//!
//! Implemented solely from the repository's clean-room behavioral
//! specification text, with no Xpra implementation source consulted by the
//! Implementation Developer. Licensed under GPL-2.0-only with zero runtime
//! dependencies.
//!
//! # Value model
//!
//! [`Value`] covers null, booleans (distinct from integers), signed 64-bit
//! integers, floats, UTF-8 text, arbitrary bytes (distinct from text), ordered
//! lists, and insertion-ordered maps ([`Map`]). Integers beyond `i64` exist in
//! the wire format's big decimal form; decoding one reports a structured
//! out-of-range error rather than panicking or misreading it.
//!
//! # Encoding and decoding
//!
//! [`encode`] emits the canonical shortest wire form and is infallible.
//! [`decode`] is strict: it decodes exactly one top-level value and rejects
//! trailing bytes, which is the right behavior at Xpra packet boundaries
//! where the packet header already gives an exact payload length. The
//! opt-in lenient variant [`decode_first`] accepts the first value and
//! returns the consumed byte count. Both accept well-formed non-canonical
//! encodings (wider integer forms, long text form for short strings,
//! variable-length containers below the fixed-form thresholds, leading
//! zeroes in decimal text).
//!
//! Decoding untrusted input is guarded by [`DecodeConfig`] limits on nesting
//! depth and per-record allocation size, and never panics: every failure is
//! a structured [`DecodeError`].
//!
//! ```
//! use rencodeplus::{decode, encode, Map, Value};
//!
//! let mut caps = Map::new();
//! caps.push("version", "6.5.1");
//! caps.push("rencodeplus", true);
//! let packet = Value::List(vec![Value::from("hello"), Value::Map(caps)]);
//!
//! let bytes = encode(&packet);
//! assert_eq!(decode(&bytes).unwrap(), packet);
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod decode;
mod encode;
mod error;
mod value;

pub use decode::{
    DecodeConfig, decode, decode_first, decode_first_with_config, decode_with_config,
};
pub use encode::{encode, encode_into};
pub use error::{DecodeError, DecodeErrorKind};
pub use value::{Map, Value};
