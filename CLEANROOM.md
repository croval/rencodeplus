# Clean-Room Provenance and Acceptance Record

This document records the provenance and black-box acceptance of
`croval/rencodeplus`, a Rust implementation of the Xpra `rencodeplus` payload
format. It is intended to travel with the repository as the clean-room record
for version `0.1.0`.

## Purpose

This crate implements Xpra's `rencodeplus` payload encoding under the
GPL-2.0-only license. Xpra's own implementation is GPLv3, so this crate was
produced through a two-role clean-room process.

The crate implements payload value serialization only. Xpra packet headers,
compression, chunk reassembly, transport, authentication, and packet-specific
semantics are outside this crate.

## Roles

- **Specification Author:** studied the GPLv3 Xpra behavior, wrote the
  behavioral specification, and performed black-box acceptance testing.
- **Implementation Developer:** implemented this crate from approved
  specification text and black-box feedback only.
- **Project Coordinator:** acted as the wall between the Specification Author
  and Implementation Developer, moving approved specification revisions,
  questions, artifacts, and test results without exposing implementation
  source to the Specification Author before acceptance.
- **Downstream Xpra Client:** any application or protocol layer that uses this
  crate to encode or decode Xpra packet payload values.

The Specification Author did not inspect Implementation Developer source code,
diffs, implementation design notes, or algorithms during development and
acceptance.

## Approved Inputs

The Implementation Developer's implementation inputs were restricted to:

- `SPECIFICATION.md`
- approved revisions of that same specification
- black-box behavioral acceptance reports from the Specification Author

Questions from the Implementation Developer were answered only by revising the
specification. No code, pseudocode, translated source, implementation excerpts,
or structural implementation advice was sent by the Specification Author.

The Implementation Developer attested that they did not open, fetch, search for,
or consult any rencode or rencodeplus implementation, GPL-licensed or
otherwise, while building this crate.

## Artifact Record

Version accepted:

- Crate version: `0.1.0`
- Frozen implementation state reported by the Implementation Developer: commit
  `868885d`, tag `v0.1.0`
- Intended repository: `https://github.com/croval/rencodeplus`
- License: GPL-2.0-only
- Runtime dependencies: none
- Dev-dependencies: none
- Unsafe code: forbidden by crate policy

Artifacts reviewed before source access:

- `Cargo.toml`
- `Cargo.lock`
- `CHANGELOG.md`
- `LICENSE`
- `README.md`
- `SPECIFICATION.md`
- external integration tests under `tests/`
- release binary `target/release/rencodeplus-cli`

No `src/` tree or packaged crate archive was included in the source-free
review package.

Release binaries were checked only as black-box artifacts. The canonical
release identity is the source commit and tag recorded above; binary hashes
are not treated as release authority because Rust release artifacts are not
guaranteed reproducible across build environments.

## Black-Box Interface

The acceptance-testing surface is the `rencodeplus-cli` binary:

```text
rencodeplus-cli decode [--lenient] (<hex> | - | --raw)
rencodeplus-cli encode [--raw] (<literal> | -)
rencodeplus-cli recode (<hex> | -)
rencodeplus-cli version | help
```

The full CLI contract, typed literal notation, exit codes, and stable decode
error tokens are documented in `SPECIFICATION.md` and `README.md`.

## Acceptance Testing

The Specification Author performed black-box testing against built artifacts
only.

Specification conformance passed:

- 114 specification conformance checks
- canonical encode vectors
- canonical decode vectors
- decode-only compatibility vectors
- non-canonical recode acceptance
- strict trailing-byte rejection
- lenient consumed-byte reporting
- raw-byte CLI decode path
- malformed input error behavior
- depth-limit behavior
- huge declared length behavior
- deterministic malformed-input sweeps
- additional hardened edge cases for floats, thresholds, dictionary keys,
  big decimal integer handling, malformed length prefixes, stdin handling, and
  raw encode/decode pipelines

Xpra interoperability passed:

- Xpra version: `6.5.1-r0`
- Environment: isolated Debian 13 test VM
- Xpra packet encoders enabled: `rencodeplus`, `yaml`, `none`
- `rencodeplus-cli encode --raw` produced hello payloads accepted by the real
  Xpra server.
- `rencodeplus-cli decode --raw` decoded real Xpra server responses.
- The decoded real server capability hello contained expected capability data,
  including `server_type`, `server.mode`, `packet-types`, `encoders`, and
  `rencodeplus`.
- Additional live packet sweep decoded real Xpra packets: `hello`,
  `encodings`, `new-window`, and `startup-complete`.

Captured raw server hello test data:

- Size: 7660 bytes
- SHA-256: `3b7dcba05b77e6cc6596f9e15481a8635f5457d8a325fd6063aa48d3a976b644`

This captured payload is test data, not implementation source.

## Source-Free Review Findings

The source-free review package contained no `src/` tree and no `.crate`
archive. It did include external Rust integration tests. Those tests exercise
public crate and CLI behavior and are appropriate for provenance review.

Smoke checks run against the review binary:

```text
rencodeplus-cli version
rencodeplus-cli encode '["hello", {"version": "6.5.1"}]'
rencodeplus-cli decode 'c28568656c6c6f678776657273696f6e85362e352e31'
rencodeplus-cli recode '313a61'
```

Observed outputs:

```text
rencodeplus-cli 0.1.0
c28568656c6c6f678776657273696f6e85362e352e31
["hello", {"version": "6.5.1"}]
8161
```

The review binary also decoded the captured real Xpra server hello payload
successfully when read via `decode --raw`.

## Post-Acceptance Rule

After acceptance, the Specification Author may inspect repository layout for
hygiene, licensing, metadata, README consistency, and release packaging. The
Specification Author must not provide algorithmic implementation fixes based
on GPLv3 Xpra knowledge. If future behavior bugs require implementation-level
guidance, use an untainted reviewer or re-establish a clean-room process.

## Licensing

- Crate license: GPL-2.0-only
- Runtime dependencies: none
- Dev-dependencies: none
- No unsafe code by crate policy
