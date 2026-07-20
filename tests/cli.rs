//! Integration tests for the rencodeplus-cli black-box interface: exercises
//! the tool exactly the way an external acceptance harness would.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rencodeplus-cli"))
        .args(args)
        .output()
        .expect("failed to run rencodeplus-cli")
}

fn cli_with_stdin(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rencodeplus-cli"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn rencodeplus-cli");
    child
        .stdin
        .take()
        .expect("stdin piped")
        .write_all(stdin)
        .expect("failed to write stdin");
    child.wait_with_output().expect("failed to wait for cli")
}

fn stdout_str(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout must be UTF-8")
}

fn stderr_str(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr must be UTF-8")
}

#[test]
fn decode_prints_typed_literal() {
    let out = cli(&["decode", "45"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "null\n");

    let out = cli(&["decode", "c2 01 81 61"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "[1, \"a\"]\n");

    let out = cli(&["decode", "67 85 62 79 74 65 73 32 2f 00 ff"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "{\"bytes\": hex:00ff}\n");
}

#[test]
fn encode_prints_canonical_hex() {
    let out = cli(&["encode", "null"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "45\n");

    let out = cli(&["encode", "[1, \"a\"]"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "c2018161\n");

    let out = cli(&["encode", "{\"a\": 1, \"b\": true}"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "688161018162 43".replace(' ', "") + "\n");

    let out = cli(&["encode", "hex:00ff"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "322f00ff\n");

    let out = cli(&["encode", "-0.0"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "2c8000000000000000\n");
}

#[test]
fn encode_decode_roundtrip_via_literals() {
    let literal = "[\"hello\", {\"version\": \"6.5.1\", \"caps\": [1, 2, 3], \"blob\": hex:0102fe, \"flag\": true, \"none\": null}]";
    let encoded = cli(&["encode", literal]);
    assert!(encoded.status.success());
    let hex = stdout_str(&encoded);
    let decoded = cli(&["decode", hex.trim()]);
    assert!(decoded.status.success());
    assert_eq!(stdout_str(&decoded).trim(), literal);
}

#[test]
fn strict_decode_rejects_trailing_bytes_with_exit_1() {
    let out = cli(&["decode", "4545"]);
    assert_eq!(out.status.code(), Some(1));
    let err = stderr_str(&out);
    assert!(err.contains("kind=trailing-bytes"), "stderr: {err}");
    assert!(err.contains("offset 1"), "stderr: {err}");
    assert!(stdout_str(&out).is_empty());
}

#[test]
fn lenient_decode_reports_consumed() {
    let out = cli(&["decode", "--lenient", "4545"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "null\nconsumed: 1 of 2 bytes\n");
}

#[test]
fn decode_errors_use_stable_kind_tokens() {
    let out = cli(&["decode", "2d"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr_str(&out).contains("kind=unknown-type-code"));

    let out = cli(&["decode", ""]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr_str(&out).contains("kind=empty-input"));

    // Big integer beyond i64: recognized, rejected as out of range.
    let out = cli(&[
        "decode",
        "3d 39 32 32 33 33 37 32 30 33 36 38 35 34 37 37 35 38 30 38 7f",
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr_str(&out).contains("kind=integer-out-of-range"));
}

#[test]
fn recode_canonicalizes_non_canonical_input() {
    // Long text form for "a" recodes to the fixed form.
    let out = cli(&["recode", "313a61"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "8161\n");

    // Wide integer form recodes to the single-byte form.
    let out = cli(&["recode", "3e05"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "05\n");
}

#[test]
fn hex_and_literal_input_errors_use_exit_2() {
    let out = cli(&["decode", "zz"]);
    assert_eq!(out.status.code(), Some(2));

    let out = cli(&["decode", "455"]); // odd digit count
    assert_eq!(out.status.code(), Some(2));

    let out = cli(&["encode", "9223372036854775808"]); // i64::MAX + 1
    assert_eq!(out.status.code(), Some(2));
    assert!(stderr_str(&out).contains("signed 64-bit"));

    let out = cli(&["encode", "{unclosed"]);
    assert_eq!(out.status.code(), Some(2));

    let out = cli(&["nonsense"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn stdin_hex_and_raw_modes() {
    // Hex via stdin with '-' (whitespace tolerated, as from xxd -p).
    let out = cli_with_stdin(&["decode", "-"], b"c3 43\n44 45\n");
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "[true, false, null]\n");

    // Raw bytes via stdin.
    let out = cli_with_stdin(&["decode", "--raw"], &[0xc2, 0x01, 0x81, 0x61]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "[1, \"a\"]\n");

    // Raw bytes out from encode.
    let out = cli_with_stdin(&["encode", "--raw", "-"], b"[1, \"a\"]");
    assert!(out.status.success());
    assert_eq!(out.stdout, vec![0xc2, 0x01, 0x81, 0x61]);
}

#[test]
fn float_and_escape_literals() {
    let out = cli(&["encode", "1.0"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "2c3ff0000000000000\n");

    let out = cli(&["decode", "2c3ff0000000000000"]);
    assert!(out.status.success());
    assert_eq!(stdout_str(&out), "1.0\n");

    // inf / -inf / nan round-trip through the CLI.
    for literal in ["inf", "-inf", "nan"] {
        let encoded = cli(&["encode", literal]);
        assert!(encoded.status.success(), "encode {literal}");
        let decoded = cli(&["decode", stdout_str(&encoded).trim()]);
        assert!(decoded.status.success(), "decode {literal}");
        assert_eq!(stdout_str(&decoded).trim(), literal);
    }

    // Escapes and non-ASCII text.
    let out = cli(&["encode", "\"a\\\"b\\\\c\\n\\u{e9}\""]);
    assert!(out.status.success());
    let hex = stdout_str(&out);
    let back = cli(&["decode", hex.trim()]);
    assert!(back.status.success());
    assert_eq!(stdout_str(&back).trim(), "\"a\\\"b\\\\c\\né\"");
}

#[test]
fn version_and_help() {
    let out = cli(&["version"]);
    assert!(out.status.success());
    assert!(stdout_str(&out).starts_with("rencodeplus-cli "));

    let out = cli(&["help"]);
    assert!(out.status.success());
    assert!(stdout_str(&out).contains("USAGE"));
}
