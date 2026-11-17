//! A small JSON reader and writer for the command-line tool.
//!
//! This is not a general JSON library. It maps JSON onto [`OwnedValue`] and
//! back, with these rules:
//!
//! * Integers that fit `i64` become CBOR integers. Other numbers become floats.
//! * Object keys become text keys, so an object is a CBOR map.
//! * Byte strings, non-text map keys and non-finite floats have no JSON form,
//!   so writing them is an error.

use std::fmt::Write as _;

use tpt_cbor::OwnedValue;

/// Maximum nesting depth accepted by the parser. Deeper input is rejected
/// rather than risking stack exhaustion.
const MAX_DEPTH: usize = 128;

/// Parses one JSON value. Trailing non-whitespace is an error.
pub fn parse(text: &str) -> Result<OwnedValue, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
        depth: 0,
    };
    parser.skip_ws();
    let value = parser.value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return Err(parser.error("unexpected characters after the value"));
    }
    Ok(value)
}

/// Writes a value as compact JSON.
pub fn write(value: &OwnedValue) -> Result<String, String> {
    let mut out = String::new();
    write_value(value, &mut out)?;
    Ok(out)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
}

impl Parser<'_> {
    fn error(&self, message: &str) -> String {
        format!("invalid JSON at byte {}: {message}", self.pos)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        Some(byte)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect_word(&mut self, word: &str) -> Result<(), String> {
        let end = self.pos + word.len();
        if self.bytes.get(self.pos..end) == Some(word.as_bytes()) {
            self.pos = end;
            Ok(())
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn value(&mut self) -> Result<OwnedValue, String> {
        match self.peek() {
            Some(b'n') => self.expect_word("null").map(|()| OwnedValue::Null),
            Some(b't') => self.expect_word("true").map(|()| OwnedValue::Bool(true)),
            Some(b'f') => self.expect_word("false").map(|()| OwnedValue::Bool(false)),
            Some(b'"') => self.string().map(OwnedValue::Text),
            Some(b'[') => self.nested(Self::array),
            Some(b'{') => self.nested(Self::object),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
            None => Err(self.error("unexpected end of input")),
        }
    }

    fn nested(
        &mut self,
        parse: fn(&mut Self) -> Result<OwnedValue, String>,
    ) -> Result<OwnedValue, String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error("nesting is too deep"));
        }
        let result = parse(self);
        self.depth -= 1;
        result
    }

    fn array(&mut self) -> Result<OwnedValue, String> {
        self.bump();
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.bump();
            return Ok(OwnedValue::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.bump() {
                Some(b',') => {}
                Some(b']') => return Ok(OwnedValue::Array(items)),
                _ => return Err(self.error("expected `,` or `]` in array")),
            }
        }
    }

    fn object(&mut self) -> Result<OwnedValue, String> {
        self.bump();
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.bump();
            return Ok(OwnedValue::Map(pairs));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a string key in object"));
            }
            let key = self.string()?;
            self.skip_ws();
            if self.bump() != Some(b':') {
                return Err(self.error("expected `:` after object key"));
            }
            self.skip_ws();
            let value = self.value()?;
            pairs.push((OwnedValue::Text(key), value));
            self.skip_ws();
            match self.bump() {
                Some(b',') => {}
                Some(b'}') => return Ok(OwnedValue::Map(pairs)),
                _ => return Err(self.error("expected `,` or `}` in object")),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.bump();
        let mut out: Vec<u8> = Vec::new();
        loop {
            let byte = self
                .bump()
                .ok_or_else(|| self.error("unterminated string"))?;
            match byte {
                b'"' => break,
                b'\\' => {
                    let escaped = self
                        .bump()
                        .ok_or_else(|| self.error("unterminated escape"))?;
                    let ch = match escaped {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.unicode_escape()?,
                        _ => return Err(self.error("invalid escape")),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                0x00..=0x1f => return Err(self.error("control character in string")),
                _ => out.push(byte),
            }
        }
        // The input was valid UTF-8 and every escape yields a valid character,
        // so this conversion cannot fail. It is still checked.
        String::from_utf8(out).map_err(|_| self.error("invalid UTF-8 in string"))
    }

    /// Reads the four hex digits after `\u`, joining a UTF-16 surrogate pair
    /// when the first is a high surrogate.
    fn unicode_escape(&mut self) -> Result<char, String> {
        let high = self.hex4()?;
        let code = if (0xd800..=0xdbff).contains(&high) {
            if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                return Err(self.error("high surrogate without a low surrogate"));
            }
            let low = self.hex4()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(self.error("invalid low surrogate"));
            }
            0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00)
        } else {
            high
        };
        char::from_u32(code).ok_or_else(|| self.error("invalid unicode escape"))
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let digit = self
                .bump()
                .and_then(|b| char::from(b).to_digit(16))
                .ok_or_else(|| self.error("expected four hex digits after \\u"))?;
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<OwnedValue, String> {
        let start = self.pos;
        let mut is_float = false;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.error("expected a digit")),
        }
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("expected a digit after `.`"));
            }
            self.digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("expected a digit in exponent"));
            }
            self.digits();
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.error("invalid number"))?;
        if is_float {
            text.parse::<f64>()
                .map(OwnedValue::Float)
                .map_err(|_| self.error("invalid number"))
        } else {
            text.parse::<i64>()
                .map(OwnedValue::Integer)
                .map_err(|_| self.error("integer does not fit in a 64-bit signed CBOR integer"))
        }
    }

    fn digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
    }
}

fn write_value(value: &OwnedValue, out: &mut String) -> Result<(), String> {
    match value {
        OwnedValue::Integer(i) => out.push_str(&i.to_string()),
        OwnedValue::Float(f) => {
            if !f.is_finite() {
                return Err("NaN and infinities have no JSON form".to_string());
            }
            out.push_str(&format!("{f:?}"));
        }
        OwnedValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        OwnedValue::Null => out.push_str("null"),
        OwnedValue::Text(t) => write_string(t, out),
        OwnedValue::Bytes(_) => {
            return Err("byte strings have no JSON form".to_string());
        }
        OwnedValue::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(item, out)?;
            }
            out.push(']');
        }
        OwnedValue::Map(pairs) => {
            out.push('{');
            for (i, (key, value)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                match key {
                    OwnedValue::Text(k) => write_string(k, out),
                    _ => {
                        return Err("map keys must be text to convert to JSON".to_string());
                    }
                }
                out.push(':');
                write_value(value, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
