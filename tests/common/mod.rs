#![allow(dead_code)]

/// Parses a hex string (ASCII whitespace ignored) into bytes.
pub fn hex(s: &str) -> Vec<u8> {
    let clean: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    assert!(
        clean.len().is_multiple_of(2),
        "odd-length hex string: {s:?}"
    );
    (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).expect("invalid hex digit"))
        .collect()
}

/// Formats bytes as a lowercase hex string.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
