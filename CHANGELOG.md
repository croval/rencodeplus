# Changelog

## 0.1.0 — 2026-07-10

Initial release.

- Encoder emitting the canonical shortest wire forms for null, booleans,
  signed 64-bit integers, binary64 floats, UTF-8 text, byte strings, lists,
  and insertion-ordered maps.
- Strict-by-default decoder (exactly one top-level value, trailing bytes
  rejected) with the opt-in lenient `decode_first`; liberal acceptance of
  well-formed non-canonical encodings, including binary32 floats and big
  decimal integers within the signed 64-bit range.
- Structured, non-panicking decode errors with stable machine-readable kind
  tokens and byte offsets; configurable nesting-depth and allocation limits
  for untrusted input.
- `rencodeplus-cli` black-box test tool: `decode`/`encode`/`recode`, hex and
  raw byte modes, typed literal notation, scriptable error output.
- Passed black-box acceptance: specification conformance and interoperability
  with a real Xpra 6.5.1-r0 server.
