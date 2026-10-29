//! Ordered JSON values.
use crate::{Error, Result};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A JSON value whose objects keep their key order.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, kept as its source text.
    Number(Number),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object with ordered keys.
    Object(Object),
}

/// A JSON number held as its validated source text, so printing it back
/// never changes the bytes the user wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Number(String);

impl Number {
    /// Validate `s` against the JSON number grammar and keep it verbatim.
    pub fn from_lexeme(s: &str) -> Option<Number> {
        let b = s.as_bytes();
        let mut i = 0;
        if b.first() == Some(&b'-') {
            i += 1;
        }
        match b.get(i) {
            Some(b'0') => i += 1,
            Some(b'1'..=b'9') => {
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
            }
            _ => return None,
        }
        if b.get(i) == Some(&b'.') {
            i += 1;
            let start = i;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == start {
                return None;
            }
        }
        if matches!(b.get(i), Some(b'e' | b'E')) {
            i += 1;
            if matches!(b.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            let start = i;
            while b.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == start {
                return None;
            }
        }
        (i == b.len()).then(|| Number(s.to_string()))
    }

    /// The source text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The value as `i64`, when the text is an integer in range.
    pub fn as_i64(&self) -> Option<i64> {
        self.0.parse().ok()
    }

    /// The value as `u64`, when the text is a non-negative integer in range.
    pub fn as_u64(&self) -> Option<u64> {
        self.0.parse().ok()
    }

    /// The value as a finite `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        self.0.parse::<f64>().ok().filter(|f| f.is_finite())
    }
}

impl From<i64> for Number {
    fn from(n: i64) -> Number {
        Number(n.to_string())
    }
}

impl From<u64> for Number {
    fn from(n: u64) -> Number {
        Number(n.to_string())
    }
}

/// An ordered JSON object. Keys are unique.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Object(Vec<(String, Value)>);

impl Object {
    /// An empty object.
    pub fn new() -> Object {
        Object(Vec::new())
    }

    /// The value stored under `k`.
    pub fn get(&self, k: &str) -> Option<&Value> {
        self.0.iter().find(|(key, _)| key == k).map(|(_, v)| v)
    }

    /// Mutable access to the value stored under `k`.
    pub fn get_mut(&mut self, k: &str) -> Option<&mut Value> {
        self.0.iter_mut().find(|(key, _)| key == k).map(|(_, v)| v)
    }

    /// Replace the value of `k` in place, else append; returns the old value.
    pub fn insert(&mut self, k: impl Into<String>, v: Value) -> Option<Value> {
        let k = k.into();
        match self.0.iter_mut().find(|(key, _)| *key == k) {
            Some((_, slot)) => Some(std::mem::replace(slot, v)),
            None => {
                self.0.push((k, v));
                None
            }
        }
    }

    /// Remove `k`, keeping the order of the rest.
    pub fn remove(&mut self, k: &str) -> Option<Value> {
        let i = self.0.iter().position(|(key, _)| key == k)?;
        Some(self.0.remove(i).1)
    }

    /// Keys in stored order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(k, _)| k.as_str())
    }

    /// Entries in stored order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Number of keys.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when there are no keys.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Value {
    /// A string value.
    pub fn str(s: &str) -> Value {
        Value::String(s.to_string())
    }

    /// The string content, if a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The boolean, if a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The items, if an array.
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Mutable items, if an array.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// The object, if an object.
    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// The mutable object, if an object.
    pub fn as_object_mut(&mut self) -> Option<&mut Object> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Follow `keys` through nested objects.
    pub fn path(&self, keys: &[&str]) -> Option<&Value> {
        let mut cur = self;
        for k in keys {
            cur = cur.as_object()?.get(k)?;
        }
        Some(cur)
    }

    /// Set the value at `keys`, creating missing objects. Fails when an
    /// intermediate value exists and is not an object.
    pub fn set_path(&mut self, keys: &[&str], v: Value) -> Result<()> {
        let Some((first, rest)) = keys.split_first() else {
            *self = v;
            return Ok(());
        };
        let Value::Object(obj) = self else {
            return Err(Error::Invalid(format!(
                "cannot set \"{first}\": parent is not an object"
            )));
        };
        if rest.is_empty() {
            obj.insert(*first, v);
            return Ok(());
        }
        if obj.get(first).is_none() {
            obj.insert(*first, Value::Object(Object::new()));
        }
        match obj.get_mut(first) {
            Some(child) => child.set_path(rest, v),
            None => Ok(()),
        }
    }

    /// Remove the value at `keys`. Emptied parents stay.
    pub fn remove_path(&mut self, keys: &[&str]) -> Option<Value> {
        let (last, parents) = keys.split_last()?;
        let mut cur = self;
        for k in parents {
            cur = cur.as_object_mut()?.get_mut(k)?;
        }
        cur.as_object_mut()?.remove(last)
    }

    /// Structural equality where objects compare as key sets; arrays stay ordered.
    pub fn equal_ignoring_order(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Object(a), Value::Object(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(k, v)| b.get(k).is_some_and(|o| v.equal_ignoring_order(o)))
            }
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.equal_ignoring_order(y))
            }
            _ => self == other,
        }
    }
}

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Value::Null => s.serialize_unit(),
            Value::Bool(b) => s.serialize_bool(*b),
            Value::Number(n) => match n.0.parse::<serde_json::Number>() {
                Ok(num) => num.serialize(s),
                Err(_) => s.serialize_f64(n.as_f64().unwrap_or(0.0)),
            },
            Value::String(t) => s.serialize_str(t),
            Value::Array(a) => {
                let mut seq = s.serialize_seq(Some(a.len()))?;
                for item in a {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Value::Object(o) => {
                let mut map = s.serialize_map(Some(o.len()))?;
                for (k, v) in o.iter() {
                    map.serialize_entry(k, v)?;
                }
                map.end()
            }
        }
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E: de::Error>(self, b: bool) -> std::result::Result<Value, E> {
        Ok(Value::Bool(b))
    }

    fn visit_i64<E: de::Error>(self, n: i64) -> std::result::Result<Value, E> {
        Ok(Value::Number(Number::from(n)))
    }

    fn visit_u64<E: de::Error>(self, n: u64) -> std::result::Result<Value, E> {
        Ok(Value::Number(Number::from(n)))
    }

    fn visit_f64<E: de::Error>(self, n: f64) -> std::result::Result<Value, E> {
        serde_json::Number::from_f64(n)
            .and_then(|num| Number::from_lexeme(&num.to_string()))
            .map(Value::Number)
            .ok_or_else(|| E::custom("number is not finite"))
    }

    fn visit_str<E: de::Error>(self, s: &str) -> std::result::Result<Value, E> {
        Ok(Value::String(s.to_string()))
    }

    fn visit_string<E: de::Error>(self, s: String) -> std::result::Result<Value, E> {
        Ok(Value::String(s))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> std::result::Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = seq.next_element::<Value>()? {
            out.push(v);
        }
        Ok(Value::Array(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut m: A) -> std::result::Result<Value, A::Error> {
        let mut out = Object::new();
        while let Some((k, v)) = m.next_entry::<String, Value>()? {
            if out.get(&k).is_some() {
                return Err(de::Error::custom(format!("duplicate key {k:?}")));
            }
            out.0.push((k, v));
        }
        Ok(Value::Object(out))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Value, D::Error> {
        d.deserialize_any(ValueVisitor)
    }
}

const MAX_DEPTH: usize = 256;

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl Parser<'_> {
    fn fail<T>(&self, msg: &str) -> Result<T> {
        let before = &self.src.as_bytes()[..self.pos.min(self.src.len())];
        let line = before.iter().filter(|b| **b == b'\n').count() + 1;
        let start = before
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i + 1);
        let col = String::from_utf8_lossy(&before[start..]).chars().count() + 1;
        Err(Error::Invalid(format!(
            "invalid JSON: {msg} at line {line} column {col}"
        )))
    }

    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn lit(&mut self, word: &str, v: Value) -> Result<Value> {
        if self.src[self.pos..].starts_with(word) {
            self.pos += word.len();
            Ok(v)
        } else {
            self.fail("expected a value")
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value> {
        self.ws();
        match self.peek() {
            None => self.fail("unexpected end of input"),
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b't') => self.lit("true", Value::Bool(true)),
            Some(b'f') => self.lit("false", Value::Bool(false)),
            Some(b'n') => self.lit("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => self.fail("expected a value"),
        }
    }

    fn number(&mut self) -> Result<Value> {
        let start = self.pos;
        let b = self.src.as_bytes();
        let mut i = start;
        while i < b.len() && matches!(b[i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
            i += 1;
        }
        let text = &self.src[start..i];
        let mut end = 0;
        // Longest valid prefix: the grammar stops where the lexeme stops being valid.
        for n in (1..=text.len()).rev() {
            if Number::from_lexeme(&text[..n]).is_some() {
                end = n;
                break;
            }
        }
        if end == 0 {
            return self.fail("invalid number");
        }
        let lexeme = &text[..end];
        self.pos = start + end;
        if end < text.len() {
            return self.fail("invalid number");
        }
        match Number::from_lexeme(lexeme) {
            Some(n) => Ok(Value::Number(n)),
            None => self.fail("invalid number"),
        }
    }

    fn hex4(&mut self) -> Result<u32> {
        let Some(h) = self.src.get(self.pos..self.pos + 4) else {
            return self.fail("invalid \\u escape");
        };
        if !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return self.fail("invalid \\u escape");
        }
        self.pos += 4;
        u32::from_str_radix(h, 16).or_else(|_| self.fail("invalid \\u escape"))
    }

    fn string(&mut self) -> Result<String> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let run = self.pos;
            while let Some(b) = self.peek() {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            out.push_str(&self.src[run..self.pos]);
            match self.peek() {
                None => return self.fail("unterminated string"),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let esc = self.peek();
                    self.pos += 1;
                    match esc {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'b') => out.push('\u{8}'),
                        Some(b'f') => out.push('\u{c}'),
                        Some(b'n') => out.push('\n'),
                        Some(b'r') => out.push('\r'),
                        Some(b't') => out.push('\t'),
                        Some(b'u') => {
                            let hi = self.hex4()?;
                            let cp = if (0xD800..0xDC00).contains(&hi) {
                                if !self.src[self.pos..].starts_with("\\u") {
                                    return self.fail("lone surrogate");
                                }
                                self.pos += 2;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return self.fail("lone surrogate");
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else if (0xDC00..0xE000).contains(&hi) {
                                return self.fail("lone surrogate");
                            } else {
                                hi
                            };
                            match char::from_u32(cp) {
                                Some(c) => out.push(c),
                                None => return self.fail("invalid \\u escape"),
                            }
                        }
                        _ => {
                            self.pos -= 1;
                            return self.fail("invalid escape");
                        }
                    }
                }
                Some(_) => return self.fail("control character in string"),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return self.fail("nesting too deep");
        }
        self.pos += 1;
        let mut out = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(out));
        }
        loop {
            out.push(self.value(depth)?);
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(out));
                }
                _ => return self.fail("expected `,` or `]`"),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return self.fail("nesting too deep");
        }
        self.pos += 1;
        let mut out = Object::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(out));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return self.fail("expected a string key");
            }
            let key_pos = self.pos;
            let key = self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return self.fail("expected `:`");
            }
            self.pos += 1;
            if out.get(&key).is_some() {
                self.pos = key_pos;
                return self.fail(&format!("duplicate key {key:?}"));
            }
            let v = self.value(depth)?;
            out.0.push((key, v));
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(out));
                }
                _ => return self.fail("expected `,` or `}`"),
            }
        }
    }
}

/// Parse one JSON document, byte-faithfully: numbers keep their source text,
/// object key order is kept, and duplicate keys at any depth are refused.
/// Typed `serde` use of [`Value`] is not byte-faithful for numbers; this is.
pub fn parse(text: &str) -> Result<Value> {
    let mut p = Parser { src: text, pos: 0 };
    let v = p.value(0)?;
    p.ws();
    if p.pos != text.len() {
        return p.fail("trailing characters");
    }
    Ok(v)
}

/// Parse an object; whitespace-only text is an empty object.
pub fn parse_object_or_empty(text: &str) -> Result<Object> {
    if text.trim().is_empty() {
        return Ok(Object::new());
    }
    match parse(text)? {
        Value::Object(o) => Ok(o),
        _ => Err(Error::Invalid(
            "invalid JSON: top level is not an object".to_string(),
        )),
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(out: &mut String, v: &Value, indent: Option<usize>) {
    let newline = |out: &mut String, level: usize| {
        out.push('\n');
        for _ in 0..level {
            out.push_str("  ");
        }
    };
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(n.as_str()),
        Value::String(s) => write_str(out, s),
        Value::Array(a) if a.is_empty() => out.push_str("[]"),
        Value::Object(o) if o.is_empty() => out.push_str("{}"),
        Value::Array(a) => {
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if let Some(l) = indent {
                    newline(out, l + 1);
                }
                write(out, item, indent.map(|l| l + 1));
            }
            if let Some(l) = indent {
                newline(out, l);
            }
            out.push(']');
        }
        Value::Object(o) => {
            out.push('{');
            for (i, (k, item)) in o.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if let Some(l) = indent {
                    newline(out, l + 1);
                }
                write_str(out, k);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write(out, item, indent.map(|l| l + 1));
            }
            if let Some(l) = indent {
                newline(out, l);
            }
            out.push('}');
        }
    }
}

/// Two-space indented JSON with a trailing newline, numbers verbatim.
pub fn to_pretty(v: &Value) -> String {
    let mut s = String::new();
    write(&mut s, v, Some(0));
    s.push('\n');
    s
}

/// JSON without whitespace or trailing newline, numbers verbatim.
pub fn to_compact(v: &Value) -> String {
    let mut s = String::new();
    write(&mut s, v, None);
    s
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn roundtrip_preserves_key_order() {
        let v = parse(r#"{"b":1,"a":{"z":1,"y":2}}"#).unwrap();
        assert_eq!(
            to_pretty(&v),
            "{\n  \"b\": 1,\n  \"a\": {\n    \"z\": 1,\n    \"y\": 2\n  }\n}\n"
        );
    }

    #[test]
    fn pretty_matches_json_stringify_cases() {
        assert_eq!(to_pretty(&parse("{}").unwrap()), "{}\n");
        assert_eq!(to_pretty(&parse("[]").unwrap()), "[]\n");
        assert_eq!(
            to_pretty(&parse(r#"{"a":[],"b":{}}"#).unwrap()),
            "{\n  \"a\": [],\n  \"b\": {}\n}\n"
        );
        let s = Value::str("q\" b\\ n\n u\u{1} é");
        assert_eq!(to_pretty(&s), "\"q\\\" b\\\\ n\\n u\\u0001 é\"\n");
    }

    #[test]
    fn compact_has_no_spaces() {
        let v = parse(" { \"a\" : [ 1 , 2 ] , \"b\" : null } ").unwrap();
        assert_eq!(to_compact(&v), r#"{"a":[1,2],"b":null}"#);
    }

    #[test]
    fn duplicate_key_refuses() {
        let m = parse(r#"{"a":1,"a":2}"#).unwrap_err().to_string();
        assert!(m.contains("duplicate key \"a\""), "{m}");
        let m = parse(r#"{"x":{"k":1,"k":2}}"#).unwrap_err().to_string();
        assert!(m.contains("duplicate key \"k\""), "{m}");
    }

    #[test]
    fn invalid_json_refuses_with_position() {
        let m = parse("{\n  \"a\": ,\n}").unwrap_err().to_string();
        assert!(m.contains("line 2"), "{m}");
    }

    #[test]
    fn trailing_garbage_refuses() {
        assert!(parse("{} x").is_err());
        assert!(parse("{}{}").is_err());
    }

    #[test]
    fn numbers_keep_source_text() {
        for n in [
            "-0",
            "1.0",
            "1e2",
            "1E5",
            "1e16",
            "0.000001",
            "0.1234567890123456789",
            "18446744073709551616",
            "1e20",
            "10",
            "-3",
            "1.5",
            "-1.5E+10",
        ] {
            let v = parse(n).unwrap();
            assert_eq!(to_compact(&v), n);
            assert_eq!(to_pretty(&v), format!("{n}\n"));
            let nested = parse(&format!("{{\"k\":[{n}]}}")).unwrap();
            assert_eq!(to_compact(&nested), format!("{{\"k\":[{n}]}}"));
        }
        let n = Number::from(-7_i64);
        assert_eq!(n.as_str(), "-7");
        assert_eq!(n.as_i64(), Some(-7));
        assert_eq!(Number::from(u64::MAX).as_u64(), Some(u64::MAX));
    }

    #[test]
    fn invalid_numbers_refuse() {
        for n in [
            "01", "1.", ".5", "+1", "1e", "-", "1e+", "--1", "0x1", "1.e2", "-01",
        ] {
            assert!(parse(n).is_err(), "{n} should refuse");
            assert!(parse(&format!("[{n}]")).is_err(), "[{n}] should refuse");
        }
    }

    #[test]
    fn lone_surrogate_refuses() {
        assert!(parse(r#""\ud800""#).is_err());
        assert!(parse(r#""\udc00""#).is_err());
        assert!(parse(r#""\ud800x""#).is_err());
        assert_eq!(
            parse(r#""\ud83d\ude00""#).unwrap().as_str(),
            Some("\u{1F600}")
        );
    }

    #[test]
    fn depth_limit_refuses() {
        let ok = format!("{}{}", "[".repeat(256), "]".repeat(256));
        assert!(parse(&ok).is_ok());
        let deep = format!("{}{}", "[".repeat(257), "]".repeat(257));
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn parser_reports_positions_and_escapes() {
        let m = parse("{\n  \"a\": tru\n}").unwrap_err().to_string();
        assert!(m.contains("line 2"), "{m}");
        assert_eq!(
            parse(r#""a\n\t\"\\\/\u00e9""#).unwrap().as_str(),
            Some("a\n\t\"\\/\u{e9}")
        );
        assert!(parse("\"a\nb\"").is_err());
        assert!(parse(r#""\x""#).is_err());
        assert!(parse("").is_err());
        assert!(parse(" [1, 2 ,3] ").is_ok());
        assert!(parse("[1,]").is_err());
    }

    #[test]
    fn parse_object_or_empty_cases() {
        assert!(parse_object_or_empty("").unwrap().is_empty());
        assert!(parse_object_or_empty("  \n").unwrap().is_empty());
        assert!(parse_object_or_empty("{}").unwrap().is_empty());
        assert!(matches!(
            parse_object_or_empty("[]"),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn insert_replaces_in_place_and_appends() {
        let mut o = Object::new();
        assert!(o.insert("a", Value::Bool(true)).is_none());
        o.insert("b", Value::Null);
        assert_eq!(o.insert("a", Value::Bool(false)), Some(Value::Bool(true)));
        o.insert("c", Value::Null);
        assert_eq!(o.keys().collect::<Vec<_>>(), ["a", "b", "c"]);
        assert_eq!(o.get("a"), Some(&Value::Bool(false)));
        assert_eq!(o.len(), 3);
    }

    #[test]
    fn remove_keeps_order_of_rest() {
        let mut o = parse_object_or_empty(r#"{"a":1,"b":2,"c":3}"#).unwrap();
        assert!(o.remove("b").is_some());
        assert!(o.remove("zz").is_none());
        assert_eq!(o.keys().collect::<Vec<_>>(), ["a", "c"]);
    }

    #[test]
    fn set_path_creates_and_errors_through_scalar() {
        let mut v = Value::Object(Object::new());
        v.set_path(&["a", "b", "c"], Value::Bool(true)).unwrap();
        assert_eq!(to_compact(&v), r#"{"a":{"b":{"c":true}}}"#);
        assert_eq!(v.path(&["a", "b", "c"]), Some(&Value::Bool(true)));
        assert!(v.set_path(&["a", "b", "c", "d"], Value::Null).is_err());
        assert!(v.path(&["a", "x"]).is_none());
    }

    #[test]
    fn remove_path_returns_value() {
        let mut v = parse(r#"{"a":{"b":1}}"#).unwrap();
        assert_eq!(
            v.remove_path(&["a", "b"]).map(|x| to_compact(&x)),
            Some("1".into())
        );
        assert_eq!(to_compact(&v), r#"{"a":{}}"#);
        assert!(v.remove_path(&["a", "b"]).is_none());
        assert!(v.remove_path(&[]).is_none());
    }

    #[test]
    fn equal_ignoring_order_true_for_reordered_objects_false_for_reordered_arrays() {
        let a = parse(r#"{"x":1,"y":{"p":1,"q":2}}"#).unwrap();
        let b = parse(r#"{"y":{"q":2,"p":1},"x":1}"#).unwrap();
        assert!(a.equal_ignoring_order(&b));
        assert_ne!(a, b);
        let c = parse(r#"{"a":[1,2]}"#).unwrap();
        let d = parse(r#"{"a":[2,1]}"#).unwrap();
        assert!(!c.equal_ignoring_order(&d));
        let e = parse(r#"{"a":1}"#).unwrap();
        let f = parse(r#"{"a":1,"b":2}"#).unwrap();
        assert!(!e.equal_ignoring_order(&f));
    }

    #[test]
    fn unknown_content_survives() {
        let text = r#"{
  "model": "x",
  "env": {"A": "1", "B": null},
  "permissions": {"allow": ["Bash(ls)", "Read"], "deny": [], "ask": [[1, 2], [true, null]]},
  "hooks": {
    "SessionStart": [
      {"matcher": "startup", "hooks": [{"type": "command", "command": "echo hi", "timeout": 10}]}
    ],
    "Stop": []
  },
  "statusLine": {"type": "command", "command": "x", "padding": 0},
  "flags": [true, false, null, 1.5, -2, "é"],
  "nested": {"a": {"b": {"c": {}}}}
}"#;
        let original = parse(text).unwrap();
        let mut v = original.clone();
        v.set_path(&["vibe", "extra", "k"], Value::str("v"))
            .unwrap();
        assert_ne!(v, original);
        v.remove_path(&["vibe", "extra", "k"]);
        v.remove_path(&["vibe", "extra"]);
        v.remove_path(&["vibe"]);
        assert_eq!(v, original);
        assert_eq!(parse(&to_pretty(&original)).unwrap(), original);
    }
}
