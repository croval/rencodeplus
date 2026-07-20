//! Black-box test interface for the rencodeplus crate.
//!
//! Exposes encode/decode through a scriptable command line so specification
//! conformance can be verified against the built artifact without source
//! access. See `rencodeplus-cli help` for the interface contract.

use std::io::{Read, Write};
use std::process::ExitCode;

use rencodeplus::{
    DecodeConfig, DecodeError, Map, Value, decode_first_with_config, decode_with_config, encode,
};

const USAGE: &str = "\
rencodeplus-cli - black-box test interface for the rencodeplus crate

USAGE:
    rencodeplus-cli decode [--lenient] (<hex> | - | --raw)
    rencodeplus-cli encode [--raw] (<literal> | -)
    rencodeplus-cli recode (<hex> | -)
    rencodeplus-cli version
    rencodeplus-cli help

INPUT:
    <hex>      hex string argument; case-insensitive, ASCII whitespace ignored
    -          read the same text form from stdin
    --raw      decode: read raw payload bytes from stdin instead of hex
               encode: write raw payload bytes to stdout (no trailing newline)

COMMANDS:
    decode     strict decode (exactly one value, no trailing bytes); prints
               the value as a typed literal on stdout.
               --lenient decodes the first value only and prints a second
               line `consumed: <n> of <m> bytes`.
    encode     parse a typed literal and print the canonical encoding as
               lowercase hex on stdout (or raw bytes with --raw)
    recode     strict decode then canonical re-encode; prints lowercase hex

TYPED LITERAL SYNTAX:
    null  true  false                 null and booleans
    0  -42                            integers (signed 64-bit)
    1.0  -0.0  6.5e-3  nan  inf  -inf floats
    \"text\"                            UTF-8 text; escapes \\\" \\\\ \\n \\r \\t \\u{hex}
    hex:00ff   hex:                   byte strings as hex (hex: is empty)
    [1, \"a\", hex:00]                  lists
    {\"key\": 1, 2: true}               maps (any value type as key)

EXIT CODES:
    0  success
    1  decode error; stderr gets one line:
       error: <message> at offset <n> (kind=<token>)
    2  usage, hex, or literal input error
";

struct CliError {
    message: String,
    code: u8,
}

fn usage_error(message: impl Into<String>) -> CliError {
    CliError {
        message: message.into(),
        code: 2,
    }
}

fn decode_error(err: DecodeError) -> CliError {
    CliError {
        message: format!("error: {err} (kind={})", err.kind().name()),
        code: 1,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err.message);
            ExitCode::from(err.code)
        }
    }
}

fn run(args: &[String]) -> Result<(), CliError> {
    let Some(command) = args.first() else {
        return Err(usage_error(USAGE));
    };
    match command.as_str() {
        "decode" => cmd_decode(&args[1..]),
        "encode" => cmd_encode(&args[1..]),
        "recode" => cmd_recode(&args[1..]),
        "version" | "--version" | "-V" => {
            println!("rencodeplus-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print!("{USAGE}");
            Ok(())
        }
        other => Err(usage_error(format!(
            "error: unknown command {other:?}\n\n{USAGE}"
        ))),
    }
}

struct Flags<'a> {
    lenient: bool,
    raw: bool,
    input: Option<&'a str>,
}

fn parse_flags<'a>(args: &'a [String], command: &str) -> Result<Flags<'a>, CliError> {
    let mut flags = Flags {
        lenient: false,
        raw: false,
        input: None,
    };
    for arg in args {
        match arg.as_str() {
            "--lenient" if command == "decode" => flags.lenient = true,
            "--raw" if command != "recode" => flags.raw = true,
            arg if arg.starts_with("--") => {
                return Err(usage_error(format!(
                    "error: unknown option {arg:?} for {command}"
                )));
            }
            arg if flags.input.is_none() => flags.input = Some(arg),
            arg => {
                return Err(usage_error(format!(
                    "error: unexpected extra argument {arg:?}"
                )));
            }
        }
    }
    Ok(flags)
}

fn cmd_decode(args: &[String]) -> Result<(), CliError> {
    let flags = parse_flags(args, "decode")?;
    let bytes = read_byte_input(&flags)?;
    let config = DecodeConfig::default();
    if flags.lenient {
        let (value, consumed) = decode_first_with_config(&bytes, &config).map_err(decode_error)?;
        println!("{value}");
        println!("consumed: {consumed} of {} bytes", bytes.len());
    } else {
        let value = decode_with_config(&bytes, &config).map_err(decode_error)?;
        println!("{value}");
    }
    Ok(())
}

fn cmd_encode(args: &[String]) -> Result<(), CliError> {
    let flags = parse_flags(args, "encode")?;
    if flags.raw && flags.input.is_none() {
        return Err(usage_error(
            "error: encode --raw still needs a literal argument or '-'",
        ));
    }
    let text = read_text_input(flags.input, "literal")?;
    let value = parse_literal(&text).map_err(|msg| usage_error(format!("error: {msg}")))?;
    let bytes = encode(&value);
    if flags.raw {
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|err| usage_error(format!("error: cannot write stdout: {err}")))?;
    } else {
        println!("{}", to_hex(&bytes));
    }
    Ok(())
}

fn cmd_recode(args: &[String]) -> Result<(), CliError> {
    let flags = parse_flags(args, "recode")?;
    let bytes = read_byte_input(&flags)?;
    let value = decode_with_config(&bytes, &DecodeConfig::default()).map_err(decode_error)?;
    println!("{}", to_hex(&encode(&value)));
    Ok(())
}

fn read_byte_input(flags: &Flags) -> Result<Vec<u8>, CliError> {
    if flags.raw {
        if flags.input.is_some() {
            return Err(usage_error(
                "error: --raw reads raw bytes from stdin; drop the positional argument",
            ));
        }
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .map_err(|err| usage_error(format!("error: cannot read stdin: {err}")))?;
        Ok(buf)
    } else {
        let text = read_text_input(flags.input, "hex string")?;
        from_hex(&text)
    }
}

fn read_text_input(arg: Option<&str>, what: &str) -> Result<String, CliError> {
    match arg {
        Some("-") => {
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .map_err(|err| usage_error(format!("error: cannot read stdin: {err}")))?;
            Ok(text)
        }
        Some(text) => Ok(text.to_owned()),
        None => Err(usage_error(format!(
            "error: missing {what} argument (or '-' for stdin)"
        ))),
    }
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).unwrap_or('0'));
    }
    out
}

fn from_hex(text: &str) -> Result<Vec<u8>, CliError> {
    let digits: Vec<char> = text.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if !digits.len().is_multiple_of(2) {
        return Err(usage_error(
            "error: invalid hex input: odd number of hex digits",
        ));
    }
    let mut bytes = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks(2) {
        let high = pair[0]
            .to_digit(16)
            .ok_or_else(|| usage_error(format!("error: invalid hex digit {:?}", pair[0])))?;
        let low = pair[1]
            .to_digit(16)
            .ok_or_else(|| usage_error(format!("error: invalid hex digit {:?}", pair[1])))?;
        bytes.push((high * 16 + low) as u8);
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Typed literal parser (inverse of the library's Display notation).
// ---------------------------------------------------------------------------

const MAX_LITERAL_DEPTH: usize = 512;

fn parse_literal(text: &str) -> Result<Value, String> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        pos: 0,
    };
    parser.skip_whitespace();
    let value = parser.parse_value(1)?;
    parser.skip_whitespace();
    if parser.pos != parser.chars.len() {
        return Err(format!(
            "invalid literal: unexpected trailing characters at position {}",
            parser.pos
        ));
    }
    Ok(value)
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.bump() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(format!(
                "invalid literal: expected {expected:?} but found {c:?} at position {}",
                self.pos - 1
            )),
            None => Err(format!(
                "invalid literal: expected {expected:?} but input ended"
            )),
        }
    }

    fn eat_keyword(&mut self, keyword: &str) -> bool {
        let end = self.pos + keyword.len();
        if end <= self.chars.len()
            && self.chars[self.pos..end]
                .iter()
                .copied()
                .eq(keyword.chars())
        {
            self.pos = end;
            true
        } else {
            false
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > MAX_LITERAL_DEPTH {
            return Err("invalid literal: nesting too deep".to_owned());
        }
        self.skip_whitespace();
        match self.peek() {
            None => Err("invalid literal: unexpected end of input".to_owned()),
            Some('[') => self.parse_list(depth),
            Some('{') => self.parse_map(depth),
            Some('"') => Ok(Value::Text(self.parse_quoted_string()?)),
            Some(c) if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' => {
                self.parse_number()
            }
            Some(c) if c.is_ascii_alphabetic() => {
                if self.eat_keyword("hex:") {
                    self.parse_hex_bytes()
                } else if self.eat_keyword("null") {
                    Ok(Value::Null)
                } else if self.eat_keyword("true") {
                    Ok(Value::Bool(true))
                } else if self.eat_keyword("false") {
                    Ok(Value::Bool(false))
                } else if self.eat_keyword("nan") {
                    Ok(Value::Float(f64::NAN))
                } else if self.eat_keyword("inf") {
                    Ok(Value::Float(f64::INFINITY))
                } else {
                    Err(format!(
                        "invalid literal: unknown keyword at position {}",
                        self.pos
                    ))
                }
            }
            Some(c) => Err(format!(
                "invalid literal: unexpected character {c:?} at position {}",
                self.pos
            )),
        }
    }

    fn parse_list(&mut self, depth: usize) -> Result<Value, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some(']') {
                self.pos += 1;
                break;
            }
            items.push(self.parse_value(depth + 1)?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some(']') => {}
                _ => {
                    return Err(format!(
                        "invalid literal: expected ',' or ']' at position {}",
                        self.pos
                    ));
                }
            }
        }
        Ok(Value::List(items))
    }

    fn parse_map(&mut self, depth: usize) -> Result<Value, String> {
        self.expect('{')?;
        let mut map = Map::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some('}') {
                self.pos += 1;
                break;
            }
            let key = self.parse_value(depth + 1)?;
            self.skip_whitespace();
            self.expect(':')?;
            let value = self.parse_value(depth + 1)?;
            map.push(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {}
                _ => {
                    return Err(format!(
                        "invalid literal: expected ',' or '}}' at position {}",
                        self.pos
                    ));
                }
            }
        }
        Ok(Value::Map(map))
    }

    fn parse_quoted_string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err("invalid literal: unterminated string".to_owned()),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        self.expect('{')?;
                        let mut code = 0u32;
                        let mut digits = 0;
                        while let Some(c) = self.peek() {
                            let Some(digit) = c.to_digit(16) else { break };
                            code = code
                                .checked_mul(16)
                                .and_then(|v| v.checked_add(digit))
                                .ok_or("invalid literal: \\u escape out of range")?;
                            digits += 1;
                            self.pos += 1;
                        }
                        if digits == 0 {
                            return Err(
                                "invalid literal: \\u{} needs at least one hex digit".to_owned()
                            );
                        }
                        self.expect('}')?;
                        let c = char::from_u32(code).ok_or(format!(
                            "invalid literal: \\u{{{code:x}}} is not a valid character"
                        ))?;
                        out.push(c);
                    }
                    Some(c) => {
                        return Err(format!("invalid literal: unknown escape \\{c}"));
                    }
                    None => return Err("invalid literal: unterminated escape".to_owned()),
                },
                Some(c) => out.push(c),
            }
        }
    }

    fn parse_hex_bytes(&mut self) -> Result<Value, String> {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
            self.pos += 1;
        }
        let digits = &self.chars[start..self.pos];
        if !digits.len().is_multiple_of(2) {
            return Err("invalid literal: hex: needs an even number of hex digits".to_owned());
        }
        let bytes = digits
            .chunks(2)
            .map(|pair| {
                let high = pair[0].to_digit(16).unwrap_or(0) as u8;
                let low = pair[1].to_digit(16).unwrap_or(0) as u8;
                high * 16 + low
            })
            .collect();
        Ok(Value::Bytes(bytes))
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some('-') || self.peek() == Some('+') {
            self.pos += 1;
        }
        if self.eat_keyword("inf") {
            let token: String = self.chars[start..self.pos].iter().collect();
            return Ok(Value::Float(if token.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }));
        }
        let mut is_float = false;
        while let Some(c) = self.peek() {
            match c {
                '0'..='9' => self.pos += 1,
                '.' => {
                    is_float = true;
                    self.pos += 1;
                }
                'e' | 'E' => {
                    is_float = true;
                    self.pos += 1;
                    if self.peek() == Some('+') || self.peek() == Some('-') {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        let token: String = self.chars[start..self.pos].iter().collect();
        if token.is_empty() || token == "-" || token == "+" {
            return Err(format!(
                "invalid literal: expected a number at position {start}"
            ));
        }
        if is_float {
            token
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| format!("invalid literal: bad float {token:?}"))
        } else {
            token.parse::<i64>().map(Value::Int).map_err(|_| {
                format!(
                    "invalid literal: integer {token} is outside the supported signed 64-bit range"
                )
            })
        }
    }
}
