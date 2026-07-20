# rencodeplus

Clean-room Rust encoder/decoder for the Xpra 6.5.1 `rencodeplus` payload format,
implemented solely from the behavioral specification
`SPECIFICATION.md`. No GPL-licensed source was consulted,
copied, or translated in producing this crate.

- **License:** GPL-2.0-only
- **Runtime dependencies:** none
- **Unsafe code:** none (`#![forbid(unsafe_code)]`)

The crate serializes and deserializes payload *values* only. Xpra packet
headers, compression, chunk reassembly, transport, and packet-name semantics
belong to the protocol layer that wraps this crate.

## Library usage

```rust
use rencodeplus::{decode, encode, Map, Value};

let mut caps = Map::new();               // insertion-ordered map
caps.push("version", "6.5.1");
caps.push("rencodeplus", true);
let packet = Value::List(vec![Value::from("hello"), Value::Map(caps)]);

let bytes = encode(&packet);             // canonical shortest form, infallible
let back = decode(&bytes)?;              // strict: one value, no trailing bytes
assert_eq!(back, packet);
# Ok::<(), rencodeplus::DecodeError>(())
```

Key behaviors:

- `Value` distinguishes UTF-8 **text** from arbitrary **bytes**, and booleans
  from integers. Lists preserve order; `Map` preserves insertion order, so
  encoding is deterministic.
- `decode` is **strict**: exactly one top-level value, trailing bytes are an
  error. The opt-in lenient variant `decode_first` returns the first value
  plus the consumed byte count.
- The decoder accepts well-formed non-canonical encodings (wider integer
  forms, long text form for short strings, variable-length containers below
  the fixed-form thresholds, leading zeroes in decimal text) and produces the
  same logical value as the canonical form.
- Integers are signed 64-bit. Big-decimal wire integers that fit in `i64`
  decode normally; larger ones fail with a structured
  `integer-out-of-range` error (never a panic or a misread).
- Malformed input always yields a structured `DecodeError` with a stable
  machine-readable kind token and a byte offset. The decoder never panics on
  untrusted input; `DecodeConfig` bounds nesting depth (default 64) and
  per-record allocation (default 256 MiB).

## Black-box test interface (`rencodeplus-cli`)

Built artifact: `target/release/rencodeplus-cli` (`cargo build --release`).
This is the acceptance-testing surface; it requires no source access.

```
rencodeplus-cli decode [--lenient] (<hex> | - | --raw)
rencodeplus-cli encode [--raw] (<literal> | -)
rencodeplus-cli recode (<hex> | -)
rencodeplus-cli version | help
```

- `<hex>` arguments are case-insensitive and ignore ASCII whitespace; `-`
  reads the same text from stdin. `decode --raw` reads raw payload bytes from
  stdin; `encode --raw` writes raw payload bytes to stdout (no newline).
- `decode` performs a strict decode and prints the value as a **typed
  literal** (one line, stdout). `--lenient` decodes the first value and adds
  a second line `consumed: <n> of <m> bytes`.
- `encode` parses a typed literal and prints the canonical encoding as
  lowercase hex. `recode` strictly decodes then re-encodes, so it maps any
  accepted non-canonical input to its canonical bytes.
- Typed literal syntax (unambiguous about kinds, round-trips through the
  tool): `null`, `true`, `false`, integers `-42`, floats `1.0` / `6.5e-3` /
  `-0.0` / `nan` / `inf` / `-inf`, text `"a\nb"` (escapes `\"` `\\` `\n`
  `\r` `\t` `\u{hex}`), bytes `hex:00ff` (empty: `hex:`), lists
  `[1, "a"]`, maps `{"key": 1, 2: true}` (any value type as key; entry
  order is significant).
- Exit codes: `0` success; `1` decode error, with one stderr line
  `error: <message> at offset <n> (kind=<token>)`; `2` usage/hex/literal
  input error. The kind tokens are stable: `empty-input`, `unexpected-end`,
  `unknown-type-code`, `unexpected-terminator`, `invalid-utf8`,
  `invalid-length`, `missing-length-separator`, `length-exceeds-input`,
  `unterminated-container`, `big-int-too-long`, `invalid-big-int`,
  `integer-out-of-range`, `trailing-bytes`, `depth-limit-exceeded`,
  `alloc-limit-exceeded`.

Examples:

```console
$ rencodeplus-cli encode '["hello", {"version": "6.5.1"}]'
c28568656c6c6f678776657273696f6e85362e352e31
$ rencodeplus-cli decode 'c28568656c6c6f678776657273696f6e85362e352e31'
["hello", {"version": "6.5.1"}]
$ rencodeplus-cli recode '313a61'      # non-canonical long text form for "a"
8161
$ rencodeplus-cli decode '4545'; echo "exit $?"
error: 1 trailing byte(s) after the top-level value at offset 1 (kind=trailing-bytes)
exit 1
```

## Building and testing

```console
$ cargo build --release   # produces target/release/rencodeplus-cli
$ cargo test              # spec vectors, malformed inputs, round-trips, CLI
```

Quick smoke test:

```console
$ cargo build --release
$ target/release/rencodeplus-cli version
rencodeplus-cli 0.1.0
$ target/release/rencodeplus-cli encode '["hello", {"version": "6.5.1"}]'
c28568656c6c6f678776657273696f6e85362e352e31
$ target/release/rencodeplus-cli decode 'c28568656c6c6f678776657273696f6e85362e352e31'
["hello", {"version": "6.5.1"}]
$ target/release/rencodeplus-cli recode '313a61'
8161
```

The test suite encodes and decodes every canonical vector in the clean-room
specification byte-for-byte, accepts every decode-only compatibility vector,
exercises every listed malformed-input case, and sweeps adversarial inputs
(all one- and two-byte strings, deterministic pseudo-random fuzz, byte-level
mutations of valid packets) asserting the decoder returns structured errors
rather than panicking.
