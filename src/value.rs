//! The logical value model.

use core::fmt;
use core::fmt::Write as _;

/// A rencodeplus value.
///
/// Text and bytes are distinct kinds, both in this model and on the wire.
/// Booleans are distinct from integers. Lists preserve item order. Maps
/// preserve insertion order (see [`Map`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// None / null.
    Null,
    /// Boolean.
    Bool(bool),
    /// Signed 64-bit integer.
    ///
    /// The wire format can carry larger integers in big decimal form; this
    /// crate decodes them when they fit in `i64` and reports a structured
    /// out-of-range error otherwise.
    Int(i64),
    /// IEEE-754 floating point number.
    ///
    /// Stored as `f64`. 32-bit floats on the wire are widened losslessly.
    Float(f64),
    /// UTF-8 text string.
    Text(String),
    /// Arbitrary byte string.
    Bytes(Vec<u8>),
    /// Ordered list.
    List(Vec<Value>),
    /// Insertion-ordered map.
    Map(Map),
}

impl Value {
    /// Returns `true` if this is [`Value::Null`].
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns the boolean if this is [`Value::Bool`].
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the integer if this is [`Value::Int`].
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the float if this is [`Value::Float`].
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(x) => Some(*x),
            _ => None,
        }
    }

    /// Returns the text if this is [`Value::Text`].
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the bytes if this is [`Value::Bytes`].
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Returns the items if this is [`Value::List`].
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    /// Returns the map if this is [`Value::Map`].
    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Value::Map(m) => Some(m),
            _ => None,
        }
    }
}

macro_rules! impl_from_int {
    ($($t:ty),* $(,)?) => {
        $(
            impl From<$t> for Value {
                fn from(v: $t) -> Self {
                    Value::Int(i64::from(v))
                }
            }
        )*
    };
}

impl_from_int!(i8, i16, i32, i64, u8, u16, u32);

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Float(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(f64::from(v))
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_owned())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl From<Vec<u8>> for Value {
    fn from(v: Vec<u8>) -> Self {
        Value::Bytes(v)
    }
}

impl From<&[u8]> for Value {
    fn from(v: &[u8]) -> Self {
        Value::Bytes(v.to_vec())
    }
}

impl<const N: usize> From<&[u8; N]> for Value {
    fn from(v: &[u8; N]) -> Self {
        Value::Bytes(v.to_vec())
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::List(v)
    }
}

impl From<Map> for Value {
    fn from(v: Map) -> Self {
        Value::Map(v)
    }
}

/// An insertion-ordered map of [`Value`] keys to [`Value`] values.
///
/// The wire format preserves the encoder's iteration order and does not
/// canonicalize keys, so this type stores entries as an ordered sequence of
/// pairs. Encoding a `Map` is deterministic: entries are written in the order
/// they were inserted. Decoding preserves duplicate keys in wire order;
/// lookups return the *last* matching entry, which matches the replacement
/// semantics a plain map would have applied.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Map {
    entries: Vec<(Value, Value)>,
}

impl Map {
    /// Creates an empty map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty map with room for `capacity` entries.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
        }
    }

    /// Appends an entry. Existing entries with an equal key are kept.
    pub fn push(&mut self, key: impl Into<Value>, value: impl Into<Value>) {
        self.entries.push((key.into(), value.into()));
    }

    /// Number of entries, counting duplicates.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entries in insertion order.
    pub fn entries(&self) -> &[(Value, Value)] {
        &self.entries
    }

    /// Consumes the map and returns its entries in insertion order.
    pub fn into_entries(self) -> Vec<(Value, Value)> {
        self.entries
    }

    /// Iterates over the entries in insertion order.
    pub fn iter(&self) -> core::slice::Iter<'_, (Value, Value)> {
        self.entries.iter()
    }

    /// Returns the value for `key`; the last matching entry wins.
    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Returns the value for a text key; the last matching entry wins.
    pub fn get_text(&self, key: &str) -> Option<&Value> {
        self.entries
            .iter()
            .rev()
            .find(|(k, _)| k.as_text() == Some(key))
            .map(|(_, v)| v)
    }
}

impl From<Vec<(Value, Value)>> for Map {
    fn from(entries: Vec<(Value, Value)>) -> Self {
        Self { entries }
    }
}

impl FromIterator<(Value, Value)> for Map {
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> Self {
        Self {
            entries: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for Map {
    type Item = (Value, Value);
    type IntoIter = std::vec::IntoIter<(Value, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a Map {
    type Item = &'a (Value, Value);
    type IntoIter = core::slice::Iter<'a, (Value, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl Extend<(Value, Value)> for Map {
    fn extend<I: IntoIterator<Item = (Value, Value)>>(&mut self, iter: I) {
        self.entries.extend(iter);
    }
}

/// Formats the value in the crate's typed literal notation.
///
/// The notation is unambiguous about kinds: `null`, `true`, `false`,
/// integers (`-5`), floats (always with a `.`, an exponent, or one of
/// `nan`/`inf`/`-inf`), double-quoted text with escapes, `hex:00ff` byte
/// strings, `[a, b]` lists and `{key: value}` maps. It is the output format
/// of the `rencodeplus-cli` tool and round-trips through the CLI's literal
/// parser (NaN payload bits excepted).
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(true) => f.write_str("true"),
            Value::Bool(false) => f.write_str("false"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(x) => {
                if x.is_nan() {
                    f.write_str("nan")
                } else if x.is_infinite() {
                    f.write_str(if *x > 0.0 { "inf" } else { "-inf" })
                } else {
                    // Debug formatting of f64 is the shortest representation
                    // that round-trips, and always carries a float marker
                    // ("1.0", "1e300") so it cannot be read back as an int.
                    write!(f, "{x:?}")
                }
            }
            Value::Text(s) => {
                f.write_char('"')?;
                for c in s.chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        '\n' => f.write_str("\\n")?,
                        '\r' => f.write_str("\\r")?,
                        '\t' => f.write_str("\\t")?,
                        c if (c as u32) < 0x20 || c == '\u{7f}' => {
                            write!(f, "\\u{{{:x}}}", c as u32)?
                        }
                        c => f.write_char(c)?,
                    }
                }
                f.write_char('"')
            }
            Value::Bytes(bytes) => {
                f.write_str("hex:")?;
                for byte in bytes {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
            Value::List(items) => {
                f.write_char('[')?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_char(']')
            }
            Value::Map(map) => {
                f.write_char('{')?;
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                f.write_char('}')
            }
        }
    }
}
