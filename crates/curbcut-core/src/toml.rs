//! A minimal TOML reader for the documented config subset.
//!
//! Supports tables, arrays, inline tables, strings, integers, floats, and
//! booleans. It is deliberately not a general TOML 1.0 implementation: the
//! config schema is fixed and small. Unknown syntax is rejected with a line
//! number and the offending key, never a value.
//!
//! Kept internal to `curbcut-core` so the schema can evolve without taking a
//! dependency. A general parser replaces this only when a user can author
//! arbitrary documents.

use std::collections::BTreeMap;

use crate::error::{CurbCutError, Result};

/// A parsed TOML value.
#[derive(Debug, Clone, PartialEq)]
pub enum TomlValue {
    /// A table.
    Table(BTreeMap<String, TomlValue>),
    /// An array.
    Array(Vec<TomlValue>),
    /// A string.
    String(String),
    /// An integer.
    Integer(i64),
    /// A float.
    Float(f64),
    /// A boolean.
    Bool(bool),
}

impl TomlValue {
    /// The type name, for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Table(_) => "table",
            Self::Array(_) => "array",
            Self::String(_) => "string",
            Self::Integer(_) => "integer",
            Self::Float(_) => "float",
            Self::Bool(_) => "boolean",
        }
    }

    /// Borrows a table field.
    pub fn as_table(&self, path: &str) -> Result<&BTreeMap<String, TomlValue>> {
        match self {
            Self::Table(table) => Ok(table),
            other => Err(value_err(path, "a table", other.type_name())),
        }
    }

    /// Borrows a string field.
    pub fn as_str(&self, path: &str) -> Result<&str> {
        match self {
            Self::String(value) => Ok(value),
            other => Err(value_err(path, "a string", other.type_name())),
        }
    }

    /// Borrows a boolean field.
    pub fn as_bool(&self, path: &str) -> Result<bool> {
        match self {
            Self::Bool(value) => Ok(*value),
            other => Err(value_err(path, "a boolean", other.type_name())),
        }
    }

    /// Borrows an integer field. A whole-number float is accepted.
    pub fn as_i64(&self, path: &str) -> Result<i64> {
        match self {
            Self::Integer(value) => Ok(*value),
            Self::Float(value) if value.fract() == 0.0 => Ok(*value as i64),
            other => Err(value_err(path, "an integer", other.type_name())),
        }
    }

    /// Borrows a float field. An integer is accepted.
    pub fn as_f64(&self, path: &str) -> Result<f64> {
        match self {
            Self::Float(value) => Ok(*value),
            Self::Integer(value) => Ok(*value as f64),
            other => Err(value_err(path, "a number", other.type_name())),
        }
    }

    /// Borrows an array field.
    pub fn as_array(&self, path: &str) -> Result<&[TomlValue]> {
        match self {
            Self::Array(values) => Ok(values),
            other => Err(value_err(path, "an array", other.type_name())),
        }
    }
}

/// Builds a config error with a static message and no dynamic value.
pub fn config_err(message: impl Into<String>) -> CurbCutError {
    CurbCutError::Config {
        message: message.into(),
    }
}

fn value_err(path: &str, expected: &str, found: &str) -> CurbCutError {
    config_err(format!("{path}: expected {expected}, found {found}"))
}

/// Parses a document into a root table.
///
/// # Errors
///
/// Returns [`CurbCutError::Config`] with a line number on malformed input.
pub fn parse(src: &str) -> Result<BTreeMap<String, TomlValue>> {
    Parser::new(src).parse_document()
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    line: u32,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn err<T>(&self, message: &str) -> Result<T> {
        Err(config_err(format!("line {}: {message}", self.line)))
    }

    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.bump();
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some('#') {
            while matches!(self.peek(), Some(c) if c != '\n') {
                self.bump();
            }
        }
    }

    fn skip_blank(&mut self) {
        loop {
            self.skip_inline_ws();
            self.skip_comment();
            match self.peek() {
                Some('\n') | Some('\r') => {
                    self.bump();
                }
                _ => break,
            }
        }
    }

    fn expect(&mut self, expected: char) -> Result<()> {
        match self.bump() {
            Some(c) if c == expected => Ok(()),
            Some(_) => self.err(&format!("expected `{expected}`")),
            None => self.err(&format!("expected `{expected}` at end of input")),
        }
    }

    fn end_of_line(&mut self) -> Result<()> {
        self.skip_inline_ws();
        self.skip_comment();
        match self.peek() {
            None => Ok(()),
            Some('\n') => {
                self.bump();
                Ok(())
            }
            Some('\r') => {
                self.bump();
                if self.peek() == Some('\n') {
                    self.bump();
                }
                Ok(())
            }
            Some(_) => self.err("expected end of line"),
        }
    }

    fn parse_document(&mut self) -> Result<BTreeMap<String, TomlValue>> {
        let mut root: BTreeMap<String, TomlValue> = BTreeMap::new();
        let mut current_path: Vec<String> = Vec::new();
        loop {
            self.skip_blank();
            if self.peek().is_none() {
                break;
            }
            if self.peek() == Some('[') {
                self.bump();
                let path = self.parse_key_path()?;
                self.expect(']')?;
                self.end_of_line()?;
                ensure_table(&mut root, &path)?;
                current_path = path;
            } else {
                let key = self.parse_key()?;
                self.skip_inline_ws();
                self.expect('=')?;
                self.skip_inline_ws();
                let value = self.parse_value()?;
                self.end_of_line()?;
                let mut path = current_path.clone();
                path.push(key);
                insert_value(&mut root, &path, value)?;
            }
        }
        Ok(root)
    }

    fn parse_key_path(&mut self) -> Result<Vec<String>> {
        let mut path = vec![self.parse_key()?];
        while self.peek() == Some('.') {
            self.bump();
            path.push(self.parse_key()?);
        }
        Ok(path)
    }

    fn parse_key(&mut self) -> Result<String> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                self.bump();
            } else {
                break;
            }
        }
        if start == self.pos {
            return self.err("expected a key");
        }
        Ok(self.src[start..self.pos].to_string())
    }

    fn parse_value(&mut self) -> Result<TomlValue> {
        match self.peek() {
            Some('"') => Ok(TomlValue::String(self.parse_string()?)),
            Some('[') => self.parse_array(),
            Some('{') => self.parse_inline_table(),
            Some(c) if c.is_ascii_digit() || c == '-' || c == '+' => self.parse_number(),
            Some(_) => {
                let ident = self.parse_ident()?;
                match ident.as_str() {
                    "true" => Ok(TomlValue::Bool(true)),
                    "false" => Ok(TomlValue::Bool(false)),
                    _ => self.err(&format!("unknown value `{ident}`")),
                }
            }
            None => self.err("expected a value"),
        }
    }

    fn parse_ident(&mut self) -> Result<String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphabetic() || c == '_') {
            self.bump();
        }
        if start == self.pos {
            return self.err("expected a value");
        }
        Ok(self.src[start..self.pos].to_string())
    }

    fn parse_string(&mut self) -> Result<String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return self.err("unterminated string"),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(_) => return self.err("unsupported escape"),
                    None => return self.err("unterminated escape"),
                },
                Some(c) => out.push(c),
            }
        }
    }

    fn parse_array(&mut self) -> Result<TomlValue> {
        self.expect('[')?;
        let mut values = Vec::new();
        loop {
            self.skip_blank();
            if self.peek() == Some(']') {
                self.bump();
                return Ok(TomlValue::Array(values));
            }
            values.push(self.parse_value()?);
            self.skip_blank();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {
                    self.bump();
                    return Ok(TomlValue::Array(values));
                }
                _ => return self.err("expected `,` or `]` in array"),
            }
        }
    }

    fn parse_inline_table(&mut self) -> Result<TomlValue> {
        self.expect('{')?;
        let mut table = BTreeMap::new();
        loop {
            self.skip_inline_ws();
            if self.peek() == Some('}') {
                self.bump();
                return Ok(TomlValue::Table(table));
            }
            let key = self.parse_key()?;
            self.skip_inline_ws();
            self.expect('=')?;
            self.skip_inline_ws();
            let value = self.parse_value()?;
            if table.contains_key(&key) {
                return self.err(&format!("`{key}` is defined twice"));
            }
            table.insert(key, value);
            self.skip_inline_ws();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some('}') => {
                    self.bump();
                    return Ok(TomlValue::Table(table));
                }
                _ => return self.err("expected `,` or `}` in inline table"),
            }
        }
    }

    fn parse_number(&mut self) -> Result<TomlValue> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E') {
                self.bump();
            } else {
                break;
            }
        }
        let text = &self.src[start..self.pos];
        if text.contains('.') || text.contains('e') || text.contains('E') {
            text.parse::<f64>()
                .map(TomlValue::Float)
                .map_err(|_| config_err(format!("line {}: invalid float", self.line)))
        } else {
            text.parse::<i64>()
                .map(TomlValue::Integer)
                .map_err(|_| config_err(format!("line {}: invalid integer", self.line)))
        }
    }
}

fn ensure_table(root: &mut BTreeMap<String, TomlValue>, path: &[String]) -> Result<()> {
    let mut node = root;
    for (index, segment) in path.iter().enumerate() {
        let entry = node
            .entry(segment.clone())
            .or_insert_with(|| TomlValue::Table(BTreeMap::new()));
        node = match entry {
            TomlValue::Table(table) => table,
            other => {
                return Err(config_err(format!(
                    "`{}` is already a {}",
                    path[..=index].join("."),
                    other.type_name()
                )))
            }
        };
    }
    Ok(())
}

fn insert_value(
    root: &mut BTreeMap<String, TomlValue>,
    path: &[String],
    value: TomlValue,
) -> Result<()> {
    let (last, parents) = path.split_last().ok_or_else(|| config_err("empty key"))?;
    let mut node = root;
    for (index, segment) in parents.iter().enumerate() {
        let entry = node
            .entry(segment.clone())
            .or_insert_with(|| TomlValue::Table(BTreeMap::new()));
        node = match entry {
            TomlValue::Table(table) => table,
            other => {
                return Err(config_err(format!(
                    "`{}` is already a {}",
                    parents[..=index].join("."),
                    other.type_name()
                )))
            }
        };
    }
    if node.contains_key(last) {
        return Err(config_err(format!("`{}` is defined twice", path.join("."))));
    }
    node.insert(last.clone(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(src: &str) -> TomlValue {
        parse(src)
            .expect("document parses")
            .into_iter()
            .next()
            .expect("one root key")
            .1
    }

    #[test]
    fn reads_scalars() {
        assert_eq!(parse_one("a = true"), TomlValue::Bool(true));
        assert_eq!(parse_one("a = 7"), TomlValue::Integer(7));
        assert_eq!(parse_one("a = 1.5"), TomlValue::Float(1.5));
        assert_eq!(parse_one("a = \"x\""), TomlValue::String("x".to_string()));
    }

    #[test]
    fn reads_tables_and_inline_tables() {
        let root = parse("[profile.focus]\nstop_motion = true\n").expect("parses");
        let profile = root.get("profile").and_then(|v| v.as_table("profile").ok());
        assert!(profile.is_some());
        let focus = profile
            .and_then(|p| p.get("focus"))
            .and_then(|v| v.as_table("profile.focus").ok())
            .expect("focus table");
        assert_eq!(focus.get("stop_motion"), Some(&TomlValue::Bool(true)));

        let inline = parse_one("a = { b = 1, c = \"d\" }");
        assert_eq!(
            inline,
            TomlValue::Table(BTreeMap::from([
                ("b".to_string(), TomlValue::Integer(1)),
                ("c".to_string(), TomlValue::String("d".to_string())),
            ]))
        );
    }

    #[test]
    fn reads_arrays_with_comments_and_trailing_comma() {
        let value = parse_one("a = [\n  \"x\", # first\n  \"y\",\n]");
        assert_eq!(
            value,
            TomlValue::Array(vec![
                TomlValue::String("x".to_string()),
                TomlValue::String("y".to_string()),
            ])
        );
    }

    #[test]
    fn rejects_unknown_syntax_with_a_line_number() {
        let err = parse("a = maybe\n").expect_err("unknown value");
        assert!(err.to_string().contains("line 1"), "{err}");
        assert!(err.to_string().contains("maybe"), "{err}");
    }

    #[test]
    fn rejects_duplicate_keys() {
        let err = parse("a = 1\na = 2\n").expect_err("duplicate");
        assert!(err.to_string().contains("defined twice"), "{err}");
    }

    #[test]
    fn rejects_trailing_garbage_on_a_line() {
        assert!(parse("a = 1 b = 2\n").is_err());
    }

    #[test]
    fn rejects_unterminated_string() {
        assert!(parse("a = \"x\n").is_err());
    }

    #[test]
    fn rejects_a_scalar_where_a_table_is_expected() {
        let err = parse("[a]\nb = 1\n[a.b]\nc = 2\n").expect_err("conflict");
        assert!(err.to_string().contains("already a"), "{err}");
    }
}
