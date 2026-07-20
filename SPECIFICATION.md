# Clean-Room Specification: rencodeplus

## Purpose

This document is a clean-room specification for a Rust crate that serializes and deserializes the `rencodeplus` payload format used by Xpra, an open-source remote graphics application system.

The document has been written by a **Specification Author** for an **Implementation Developer** to execute. A **Project Coordinator** carries specification revisions, questions, artifacts, and test results between those roles. A **Downstream Xpra Client** means any application or protocol layer that uses this crate to encode or decode Xpra packet payload values.

Recommended GitHub repository name: `croval/rencodeplus`

Recommended Rust crate name: `rencodeplus`

Recommended license: GPL-2.0-only.

GPL-2.0-only is chosen by the implementation copyright holder to keep this crate copyleft while remaining usable by GPLv2-compatible downstream projects. Runtime dependencies must also be GPLv2-compatible. Prefer MIT, ISC, BSD-2-Clause, BSD-3-Clause, Zlib-style, or GPL-2.0-compatible dependencies. Do not introduce Apache-2.0, MPL, LGPL, GPLv3, AGPL, or proprietary runtime dependencies without explicit review and approval.

This specification describes the externally observable wire format needed for Xpra packet encoding compatibility. It must not be treated as permission to copy or inspect GPLv3 implementation code. The Implementation Developer should receive this document, approved test vectors, and black-box interoperability requirements only.

Important recipient rule: the Implementation Developer must not inspect Xpra's GPLv3 `rencodeplus` source, translated versions of that source, implementation notes derived from that source, or any code written by the Specification Author.

## Clean-Room Boundary

Specification Author's role:
- Study existing behavior of Xpra 6.5.1.
- Write this behavioral specification.
- Produce byte-level test vectors and interoperability expectations.
- Later run black-box tests against the Implementation Developer's output.
- Do not write or correct any implementation code for the crate.

Implementation Developer's role:
- Implement from this specification only.
- Do not inspect Xpra's GPLv3 `rencodeplus` implementation.
- Do not receive excerpts of that implementation.
- Do not ask the Specification Author for implementation structure or pseudocode.
- Choose the public API shape, internal representation, algorithms, and module layout freely, provided the externally observable behavior, test vectors, validation rules, and interoperability requirements in this specification pass.

This document intentionally specifies data model, byte format, validation behavior, test vectors, and interoperability scope. It does not specify algorithms, internal data structures, or code organization.

## Scope

`rencodeplus` is only a serializer/deserializer for the Xpra `rencodeplus` payload format.

In scope:
- Encode and decode the scalar and container types listed below.
- Preserve the distinction between UTF-8 text strings and arbitrary byte strings.
- Produce byte output compatible with Xpra 6.5.1 for values in this specification.
- Decode Xpra 6.5.1 packet payload values required by a Downstream Xpra Client.
- Provide enough validation and error reporting for malformed remote packets.

Out of scope:
- Xpra packet headers.
- Xpra compression.
- Xpra chunk reassembly.
- SSH/TCP transport.
- Xpra hello/capability negotiation.
- Xpra clipboard semantics.
- Xpra image or video codecs.
- Any GUI or rendering behavior.

Context only: in Xpra protocol packets, `rencodeplus` payloads are carried inside Xpra's 8-byte packet framing with protocol flag `16`. That framing belongs to a Downstream Xpra Client's protocol layer, not this crate.

## Data Model

The crate must support these logical value types:
- `None` / null.
- Boolean.
- Signed integer.
- Floating point number.
- UTF-8 text string.
- Arbitrary byte string.
- Ordered list / array.
- Dictionary / map.

The Rust API may choose its own public enum names, but it must expose text and bytes as distinct value kinds.

Sequence compatibility note:
- Xpra encodes Python sequence-like values as list records.
- Xpra decodes list records as Python tuples.
- The Rust crate should expose these as one neutral array/list value. It does not need a separate tuple type.

Map ordering note:
- The wire format preserves the iteration order used by the encoder.
- The format itself does not canonicalize dictionary keys.
- Byte-for-byte encoding tests involving dictionaries must use an insertion-preserving map or another explicitly ordered representation.
- Semantic decode tests should not require dictionary item order unless the test says it is an ordered-map encoding test.

## Type Codes

All type codes are single unsigned bytes.
| Meaning | Decimal | Hex |
|---|---:|---:|
| 64-bit float | 44 | `2c` |
| Variable-length list | 59 | `3b` |
| Variable-length dictionary | 60 | `3c` |
| Big decimal integer | 61 | `3d` |
| 1-byte signed integer | 62 | `3e` |
| 2-byte signed integer | 63 | `3f` |
| 4-byte signed integer | 64 | `40` |
| 8-byte signed integer | 65 | `41` |
| 32-bit float | 66 | `42` |
| Boolean true | 67 | `43` |
| Boolean false | 68 | `44` |
| None/null | 69 | `45` |
| Terminator | 127 | `7f` |

Fixed positive integers:
- Type codes `0` through `43`, hex `00` through `2b`.
- The type code itself is the integer value.

Fixed negative integers:
- Type codes `70` through `101`, hex `46` through `65`.
- `46` means `-1`.
- `47` means `-2`.
- Continue through `65`, which means `-32`.

Fixed dictionaries:
- Type codes `102` through `126`, hex `66` through `7e`.
- Dictionary item count is `type_code - 102`.
- Each item is encoded as key followed by value.

Fixed UTF-8 strings:
- Type codes `128` through `191`, hex `80` through `bf`.
- Byte length is `type_code - 128`.
- The following bytes are UTF-8 text bytes.

Fixed lists:
- Type codes `192` through `255`, hex `c0` through `ff`.
- List item count is `type_code - 192`.
- Each item follows immediately.

## Integer Encoding
All multi-byte binary integers are signed and big-endian.

The encoder must choose the shortest compatible integer form according to this order:

| Value range | Encoding |
|---|---|
| `0` through `43` | fixed positive integer type code |
| `-1` through `-32` | fixed negative integer type code |
| remaining values from `-128` through `127` | `3e` followed by one signed byte |
| remaining values from `-32768` through `32767` | `3f` followed by two signed big-endian bytes |
| remaining values from `-2147483648` through `2147483647` | `40` followed by four signed big-endian bytes |
| remaining values from `-9223372036854775808` through `9223372036854775807` | `41` followed by eight signed big-endian bytes |
| values outside signed 64-bit range | `3d`, decimal ASCII integer text, then `7f` |

Big decimal integer rules:
- The decimal text is ASCII.
- A negative value includes a leading minus sign.
- No leading plus sign is used.
- Encoders must not emit decimal integer text of 64 bytes or more.
- Decoders must reject decimal integer text of 64 bytes or more.
- For a Downstream Xpra Client using this crate, signed 64-bit integers are expected to be sufficient in normal protocol traffic. The crate may expose larger integers through a feature or reject out-of-range decoded big integers with a clear error, but it must at least detect the `3d` form and fail explicitly rather than treating it as an unknown type.

## Floating Point Encoding
The encoder emits 64-bit floats:
- Type code `2c`.
- Followed by eight IEEE-754 binary64 bytes in big-endian order.

The decoder must accept 64-bit floats in that form.

The decoder should also accept 32-bit floats:
- Type code `42`.
- Followed by four IEEE-754 binary32 bytes in big-endian order.

The encoder does not need to emit 32-bit floats.

Floating point values are not expected to be common in the Xpra control path of a Downstream Xpra Client using this crate, but they are part of the observable format. The decoder should preserve normal IEEE behavior for infinities, negative zero, and NaN values. Tests for NaN should compare with an `is_nan` style semantic check, not byte equality after round-trip unless the test explicitly fixes the NaN payload.

## Text and Bytes
Text strings and byte strings are distinct on the wire.

Text strings:
- Encoded from UTF-8 bytes.
- If the UTF-8 byte length is less than `64`, use fixed string form:
  - one type code from `80` through `bf`;
  - type code is `128 + byte_length`;
  - followed by the UTF-8 bytes.
- If the UTF-8 byte length is `64` or greater, use long text form:
  - ASCII decimal byte length;
  - colon byte `3a`;
  - UTF-8 bytes.

Byte strings:
- Always use binary length form, including empty and short byte strings.
- Encoding is:
  - ASCII decimal byte length;
  - slash byte `2f`;
  - raw bytes.

Decoder rules:
- Fixed string type codes always produce text and must be valid UTF-8.
- A digit-prefixed length followed by colon produces text and must be valid UTF-8.
- A digit-prefixed length followed by slash produces bytes and does not require UTF-8.
- The byte length counts bytes, not Unicode scalar values or display characters.

## Lists
Fixed list form:
- For lengths `0` through `63`, encode one type code `192 + length`.
- Encode each item immediately after the type code.

Variable-length list form:
- For lengths `64` or greater, encode type code `3b`.
- Encode each item in order.
- End with terminator `7f`.

Decoder rules:
- Fixed list length comes from the type code.
- Variable-length list continues until terminator `7f`.
- End of input before the terminator is malformed input.

## Dictionaries
Fixed dictionary form:
- For item counts `0` through `24`, encode one type code `102 + item_count`.
- Encode each key immediately followed by its value.

Variable-length dictionary form:
- For item counts `25` or greater, encode type code `3c`.
- Encode each key immediately followed by its value.
- End with terminator `7f`.

Decoder rules:
- Fixed dictionary item count comes from the type code.
- Variable-length dictionary continues key/value pairs until terminator `7f`.
- End of input before a key, before a value, or before the terminator is malformed input.
- Duplicate keys should use normal map semantics chosen by the Rust API. If a standard map is used, later values may replace earlier values. If an ordered multimap is exposed, duplicates may be preserved. Protocol traffic for a Downstream Xpra Client based on this crate is not expected to depend on duplicate keys.

## None and Booleans
None/null:

- Encoded as single byte `45`.

Boolean true:

- Encoded as single byte `43`.

Boolean false:

- Encoded as single byte `44`.

Booleans must remain distinct from integers in the public value model.

## Decoding Completeness

For packet payload use, the recommended default decode behavior is strict:

- Decode exactly one top-level value.
- Report an error if non-empty trailing bytes remain.

Compatibility note:

- Xpra's low-level load behavior accepts the first value and may ignore trailing bytes.
- A Downstream Xpra Client using this crate should prefer strict decoding at packet boundaries because the Xpra packet header already provides an exact payload length.
- If the crate exposes an Xpra-compatible lenient mode, it must be opt-in and clearly named.

Canonical encoding and liberal decoding:

- Encoders must emit the canonical shortest forms described in this specification.
- Decoders should accept well-formed non-canonical encodings unless this document explicitly says to reject them.
- Strict decoding means the input contains exactly one complete top-level value with no trailing bytes. It does not mean the input must use the shortest possible wire form.
- Examples of well-formed non-canonical input that decoders should accept: wider-than-necessary integer forms, long text form for strings shorter than 64 bytes, and variable-length list or dictionary forms below the fixed-form thresholds.
- When decoding non-canonical forms, the resulting logical value is the same as the value produced by the canonical form.

Decimal text compatibility:

- Digit-prefixed string and byte lengths may contain leading zeroes.
- Big decimal integer text may contain leading zeroes after an optional minus sign.
- Big decimal integer text `-0` is accepted and decodes to integer zero.
- Encoders should still emit canonical decimal text: no leading zeroes except the single digit `0`.

## Malformed Input Requirements

The decoder must reject malformed input with explicit errors. It must not panic on untrusted network bytes.

Malformed cases include:

- Empty input.
- Unknown type code.
- Truncated integer, float, string, bytes, list, or dictionary.
- Invalid UTF-8 in a text string.
- Digit-prefixed length with no colon or slash before end of input.
- Digit-prefixed length that is not a valid non-negative decimal byte length.
- Declared string or byte length larger than remaining input.
- Variable-length list or dictionary without a terminator.
- Big decimal integer text of 64 bytes or more.
- Big decimal integer text that is not a valid signed decimal integer.
- Integer value outside the crate's supported integer range, if the crate does not expose arbitrary-size integers.

The decoder should also enforce a configurable maximum nesting depth and maximum allocation size. Defaults should be conservative enough for untrusted network input but large enough for ordinary Xpra packets. These limits are API safety controls, not wire-format features.

## Minimal API Expectations

The exact Rust API is the Implementation Developer's design decision, but the crate should provide these capabilities:

- Encode a value to bytes.
- Decode bytes to a value.
- Strict decode by default.
- Optional lenient decode only if useful for compatibility testing.
- Distinguish text and bytes.
- Preserve list item order.
- Offer deterministic dictionary encoding when given an ordered dictionary representation.
- Return structured errors rather than panicking.
- Be usable without unsafe Rust unless the Implementation Developer has a compelling reason and the Project Coordinator approves it.
- Provide a black-box test interface suitable for Specification Author acceptance testing without source-code access. A small command-line tool is acceptable. The Implementation Developer should propose the exact interface contract for approval before relying on it for acceptance testing.

The crate does not need Serde integration for the first release. Serde support may be added later only if it does not complicate the clean-room implementation or introduce license issues.

## Approved Black-Box CLI Contract

The Implementation Developer may provide a command-line artifact named `rencodeplus-cli` for black-box acceptance testing. The Specification Author may test this binary without source-code access.

Approved commands:

- `rencodeplus-cli decode [--lenient] (<hex> | - | --raw)`
- `rencodeplus-cli encode [--raw] (<literal> | -)`
- `rencodeplus-cli recode (<hex> | -)`
- `rencodeplus-cli version`
- `rencodeplus-cli help`

CLI behavior:

- Hex arguments are case-insensitive and ignore ASCII whitespace.
- `-` reads the same text form from standard input.
- `decode --raw` reads raw payload bytes from standard input.
- `encode --raw` writes raw payload bytes to standard output.
- `decode` performs strict decode: exactly one top-level value and no trailing bytes.
- `decode` prints one line containing the value in typed literal notation.
- `decode --lenient` decodes the first value only and also prints `consumed: <n> of <m> bytes`.
- `encode` parses typed literal notation and prints canonical lowercase hex.
- `recode` performs strict decode followed by canonical re-encode.

Typed literal notation:

- Null: `null`.
- Booleans: `true`, `false`.
- Integers: decimal signed integer text, for example `-42`.
- Floats: decimal and exponent forms such as `1.0`, `6.5e-3`, `-0.0`, plus `nan`, `inf`, and `-inf`.
- Text: quoted strings with escapes for quote, backslash, newline, carriage return, tab, and Unicode scalar values.
- Bytes: `hex:` followed by hex bytes; bare `hex:` means empty bytes.
- Lists: bracketed comma-separated values.
- Maps: brace-delimited key/value pairs with significant entry order and any value type as key.

Exit codes:

- `0`: success.
- `1`: decode error, with one standard-error line in the form `error: <message> at offset <n> (kind=<token>)`.
- `2`: usage, hex parsing, or literal parsing error.

Stable error-kind tokens:

- `empty-input`
- `unexpected-end`
- `unknown-type-code`
- `unexpected-terminator`
- `invalid-utf8`
- `invalid-length`
- `missing-length-separator`
- `length-exceeds-input`
- `unterminated-container`
- `big-int-too-long`
- `invalid-big-int`
- `integer-out-of-range`
- `trailing-bytes`
- `depth-limit-exceeded`
- `alloc-limit-exceeded`

## Canonical Test Vectors

Hex strings below are byte-for-byte expected encodings. Spaces are visual separators only.

### Scalars

| Value | Expected hex |
|---|---|
| None/null | `45` |
| true | `43` |
| false | `44` |
| integer `0` | `00` |
| integer `1` | `01` |
| integer `43` | `2b` |
| integer `44` | `3e 2c` |
| integer `127` | `3e 7f` |
| integer `128` | `3f 00 80` |
| integer `32767` | `3f 7f ff` |
| integer `32768` | `40 00 00 80 00` |
| integer `2147483647` | `40 7f ff ff ff` |
| integer `2147483648` | `41 00 00 00 00 80 00 00 00` |
| integer `-1` | `46` |
| integer `-2` | `47` |
| integer `-32` | `65` |
| integer `-33` | `3e df` |
| integer `-128` | `3e 80` |
| integer `-129` | `3f ff 7f` |
| integer `-32768` | `3f 80 00` |
| integer `-32769` | `40 ff ff 7f ff` |
| integer `-2147483648` | `40 80 00 00 00` |
| integer `-2147483649` | `41 ff ff ff ff 7f ff ff ff` |
| float `1.0` | `2c 3f f0 00 00 00 00 00 00` |
| float `-0.0` | `2c 80 00 00 00 00 00 00 00` |

### Text

| Value | Expected hex |
|---|---|
| text empty string | `80` |
| text `a` | `81 61` |
| text `hello` | `85 68 65 6c 6c 6f` |
| text `é` | `82 c3 a9` |
| text of 64 ASCII `a` bytes | `36 34 3a` followed by 64 bytes of `61` |

### Bytes

| Value | Expected hex |
|---|---|
| empty bytes | `30 2f` |
| bytes `61` | `31 2f 61` |
| bytes `68 65 6c 6c 6f` | `35 2f 68 65 6c 6c 6f` |
| bytes `00 ff` | `32 2f 00 ff` |

### Lists

| Value | Expected hex |
|---|---|
| empty list | `c0` |
| list `[1]` | `c1 01` |
| list `[1, text "a"]` | `c2 01 81 61` |
| list `[true, false, None/null]` | `c3 43 44 45` |
| list containing empty list | `c1 c0` |
| list of 64 zeros | `3b` followed by 64 bytes of `00`, then `7f` |

### Dictionaries

Dictionary vectors assume the item order shown.

| Value | Expected hex |
|---|---|
| empty dictionary | `66` |
| dictionary `{text "a": integer 1}` | `67 81 61 01` |
| dictionary `{text "a": integer 1, text "b": true}` | `68 81 61 01 81 62 43` |
| dictionary `{text "bytes": bytes 00 ff}` | `67 85 62 79 74 65 73 32 2f 00 ff` |

### Big Integer Form

| Value | Expected hex |
|---|---|
| integer `9223372036854775808` | `3d 39 32 32 33 33 37 32 30 33 36 38 35 34 37 37 35 38 30 38 7f` |

If the initial crate supports only signed 64-bit integers, this vector may be placed in a decoder-recognition test that verifies a clear out-of-range error. If the crate supports larger integers, it should round-trip this value.

### Decode-Only Compatibility Vectors

The decoder should accept these forms even if the encoder does not emit them:

| Wire value | Expected decoded value |
|---|---|
| `42 3f 80 00 00` | float `1.0` from binary32 |
| `31 3a 61` | text `a` in long text form |
| `30 3a` | empty text in long text form |

## Xpra Interoperability Requirements

The crate passes acceptance when all of these are true:

1. It passes the canonical byte-level tests in this document.
2. It round-trips the supported value set without changing text into bytes or bytes into text.
3. It rejects malformed input without panics.
4. It can encode a minimal Xpra `hello` packet value for a Downstream Xpra Client's protocol layer to wrap in an Xpra packet header.
5. It can decode the server's Xpra 6.5.1 `hello` response value when the server speaks `rencodeplus`.
6. It can decode realistic Xpra control packets containing nested dictionaries, arrays, strings, bytes, booleans, and integers.
7. It can decode draw-related packet metadata while leaving large raw payload chunk handling to the Xpra protocol layer.

The crate itself should not know about Xpra packet names. Packet names such as `hello`, `draw`, clipboard packet names, and codec packet names are values carried by the serializer, not serializer concepts.

## Black-Box Acceptance Testing

After the Implementation Developer produces the GPL-2.0-only implementation, the Specification Author may test only built artifacts, command-line tools, libraries exposed through a compiled test harness, or packaged release outputs. The Specification Author must not inspect the Implementation Developer's source code, implementation notes, diffs, or internal design.

Specification Author's allowed testing role:

- Run black-box tests against the Implementation Developer's artifact.
- Compare externally observed behavior against this specification.
- Test interoperability against a real Xpra server.
- Report failures only as behavioral observations.

Specification Author must not:

- Read the Implementation Developer's source code.
- Suggest implementation fixes.
- Provide pseudocode or algorithms.
- Rewrite failing parts.
- Explain how to restructure the implementation.

### Specification Conformance Tests

This test set checks the artifact against this document without using an Xpra server.

Required checks:

- Encode each canonical value and compare the exact emitted bytes with the expected hex vectors.
- Decode each canonical byte vector and compare the resulting logical value.
- Verify text and bytes remain distinct.
- Verify list item order is preserved.
- Verify dictionary vectors that specify item order produce the expected byte order when using an ordered input representation.
- Verify strict decode rejects trailing bytes.
- Verify malformed input cases return errors rather than panicking or crashing.
- Verify decode-only compatibility vectors are accepted.
- Verify unsupported or out-of-range values fail with clear errors when the implementation intentionally does not support them.

Failure reports must use external facts only, for example:

- The input value.
- The expected bytes or expected decoded value.
- The actual bytes, actual decoded value, or observed error.
- Whether the process exited normally, returned an error, crashed, or timed out.

### Xpra Interoperability Tests

This test set checks whether the artifact can interoperate with a real Xpra server through the Xpra packet protocol. The rencodeplus artifact is still only responsible for payload serialization. Xpra packet headers, compression, chunk handling, TCP, and SSH forwarding may be supplied by a separate test harness.

The Specification Author may add approved real-world payload vectors captured from an Xpra server, provided those vectors are raw packet data and not source code, translated source, or implementation notes. Such vectors may include a server `hello` payload after the Xpra protocol layer has stripped framing and compression.

Required checks:

- Start a real Xpra server.
- Encode a minimal client `hello` payload using the black-box artifact.
- Wrap that payload in a valid Xpra packet header with the `rencodeplus` protocol flag.
- Send it over the existing TCP-over-SSH-forwarded connection path.
- Confirm the server accepts the packet instead of disconnecting with a packet-encoding or protocol error.
- Receive the server `hello` packet.
- Decode the server `hello` payload using the Implementation Developer's artifact.
- Verify the decoded value has the expected top-level packet shape and contains ordinary Xpra capability data.
- Decode representative subsequent control packets when available.
- Decode draw-related packet metadata when available, leaving raw payload chunk handling outside the crate.

Failure reports must stay at the behavior level, for example:

- The server disconnected after the client `hello`.
- The server logged a packet encoding error.
- The server `hello` payload could not be decoded.
- The decoded server packet had an unexpected top-level shape.
- A specific packet payload failed to decode with a specific external error.

These reports must not include implementation advice. The Implementation Developer remains responsible for diagnosing and fixing their implementation from the specification and observed failures.

## Non-Goals for the First Crate Release

The first release does not need:

- Schema validation for Xpra packets.
- Automatic conversion into third-party-specific protocol structs.
- Serde integration.
- Arbitrary precision integer support, provided out-of-range big integers fail clearly.
- Encoding of binary32 floats.
- Compression.
- Async I/O.
- Network code.

## Review Checklist for Implementation Developer's Submission

Before accepting the crate, verify:

- The repo license is GPL-2.0-only.
- Dependency licenses are GPLv2-compatible.
- No GPLv3, LGPLv3, AGPL, Apache-2.0, or MPL runtime dependency is present without explicit approval.
- The repository contains no copied Xpra GPLv3 code or translated implementation.
- The public API distinguishes text from bytes.
- Decode errors are structured and do not panic.
- Strict decoding rejects trailing bytes.
- All canonical vectors pass.
- The crate can be used by a small Xpra handshake probe over any existing SSH-forwarded TCP connection.
