//! 极简 JSON 实现（解析 / 序列化 / 路径读写）
//! Minimal JSON implementation (parse / serialize / path access).
//! 目的：避免引入第三方依赖，同时满足配置、API 与日志的结构化需求。

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub fn obj() -> Json {
        Json::Obj(BTreeMap::new())
    }

    pub fn str<S: Into<String>>(v: S) -> Json {
        Json::Str(v.into())
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Json> {
        match self {
            Json::Obj(m) => m.get_mut(key),
            _ => None,
        }
    }

    /// 设置键值，自动把非对象转换为对象
    pub fn set<S: Into<String>>(&mut self, key: S, val: Json) -> &mut Json {
        if !matches!(self, Json::Obj(_)) {
            *self = Json::obj();
        }
        if let Json::Obj(m) = self {
            m.insert(key.into(), val);
        }
        self
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            Json::Str(s) => s.trim().parse::<f64>().ok(),
            Json::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64().map(|n| n as i64)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            Json::Num(n) => Some(*n != 0.0),
            Json::Str(s) => match s.to_ascii_lowercase().as_str() {
                "true" | "1" | "on" | "yes" => Some(true),
                "false" | "0" | "off" | "no" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }

    /// 点分路径读取，如 "launch.ctxSize"
    pub fn path(&self, path: &str) -> Option<&Json> {
        let mut cur = self;
        for seg in path.split('.').filter(|s| !s.is_empty()) {
            cur = cur.get(seg)?;
        }
        Some(cur)
    }

    /// 点分路径写入，缺失的中间对象自动创建
    pub fn set_path(&mut self, path: &str, val: Json) {
        let segs: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        if segs.is_empty() {
            return;
        }
        let mut cur = self;
        for seg in &segs[..segs.len() - 1] {
            if !matches!(cur.get(seg), Some(Json::Obj(_))) {
                cur.set(*seg, Json::obj());
            }
            cur = cur.get_mut(seg).expect("just created");
        }
        cur.set(segs[segs.len() - 1], val);
    }

    pub fn s(&self, path: &str, def: &str) -> String {
        self.path(path)
            .and_then(|v| v.as_str())
            .unwrap_or(def)
            .to_string()
    }

    pub fn i(&self, path: &str, def: i64) -> i64 {
        self.path(path).and_then(|v| v.as_i64()).unwrap_or(def)
    }

    pub fn b(&self, path: &str, def: bool) -> bool {
        self.path(path).and_then(|v| v.as_bool()).unwrap_or(def)
    }

    pub fn to_compact(&self) -> String {
        let mut out = String::new();
        self.write_into(&mut out, None, 0);
        out
    }

    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write_into(&mut out, Some(2), 0);
        out
    }

    fn write_into(&self, out: &mut String, indent: Option<usize>, depth: usize) {
        let pad = |out: &mut String, d: usize| {
            if let Some(n) = indent {
                out.push('\n');
                for _ in 0..n * d {
                    out.push(' ');
                }
            }
        };
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            Json::Str(s) => out.push_str(&escape(s)),
            Json::Arr(a) => {
                out.push('[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                        if indent.is_none() {
                            out.push(' ');
                        }
                    }
                    pad(out, depth + 1);
                    v.write_into(out, indent, depth + 1);
                }
                if !a.is_empty() {
                    pad(out, depth);
                }
                out.push(']');
            }
            Json::Obj(m) => {
                out.push('{');
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                        if indent.is_none() {
                            out.push(' ');
                        }
                    }
                    pad(out, depth + 1);
                    out.push_str(&escape(k));
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write_into(out, indent, depth + 1);
                }
                if !m.is_empty() {
                    pad(out, depth);
                }
                out.push('}');
            }
        }
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn parse(text: &str) -> Result<Json, String> {
    let bytes: Vec<char> = text.chars().collect();
    let mut p = Parser { b: &bytes, i: 0 };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i < p.b.len() {
        return Err(format!("trailing data at {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [char],
    i: usize,
}

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.b.get(self.i).copied()
    }

    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            None => Err("unexpected end".into()),
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('t') => self.lit("true", Json::Bool(true)),
            Some('f') => self.lit("false", Json::Bool(false)),
            Some('n') => self.lit("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            Some(c) => Err(format!("unexpected char {:?} at {}", c, self.i)),
        }
    }

    fn lit(&mut self, word: &str, val: Json) -> Result<Json, String> {
        let end = self.i + word.chars().count();
        if end > self.b.len() {
            return Err(format!("bad literal at {}", self.i));
        }
        let got: String = self.b[self.i..end].iter().collect();
        if got != word {
            return Err(format!("bad literal {:?} at {}", got, self.i));
        }
        self.i = end;
        Ok(val)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        if self.peek() == Some('-') {
            self.i += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.i += 1;
        }
        if self.peek() == Some('.') {
            self.i += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            self.i += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.i += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.i += 1;
            }
        }
        let s: String = self.b[start..self.i].iter().collect();
        s.parse::<f64>()
            .map(Json::Num)
            .map_err(|_| format!("bad number {:?} at {}", s, start))
    }

    fn string(&mut self) -> Result<String, String> {
        if self.peek() != Some('"') {
            return Err(format!("expected string at {}", self.i));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or_else(|| "unterminated string".to_string())?;
            self.i += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = self.peek().ok_or_else(|| "bad escape".to_string())?;
                    self.i += 1;
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let mut code = 0u32;
                            for _ in 0..4 {
                                let h = self.peek().ok_or_else(|| "bad \\u".to_string())?;
                                self.i += 1;
                                code = code * 16 + h.to_digit(16).ok_or_else(|| "bad hex".to_string())?;
                            }
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        other => return Err(format!("bad escape \\{}", other)),
                    }
                }
                c => out.push(c),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.i += 1;
        let mut arr = Vec::new();
        self.ws();
        if self.peek() == Some(']') {
            self.i += 1;
            return Ok(Json::Arr(arr));
        }
        loop {
            self.ws();
            arr.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some(']') => {
                    self.i += 1;
                    return Ok(Json::Arr(arr));
                }
                _ => return Err(format!("expected , or ] at {}", self.i)),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.i += 1;
        let mut map = BTreeMap::new();
        self.ws();
        if self.peek() == Some('}') {
            self.i += 1;
            return Ok(Json::Obj(map));
        }
        loop {
            self.ws();
            let k = self.string()?;
            self.ws();
            if self.peek() != Some(':') {
                return Err(format!("expected : at {}", self.i));
            }
            self.i += 1;
            self.ws();
            let v = self.value()?;
            map.insert(k, v);
            self.ws();
            match self.peek() {
                Some(',') => {
                    self.i += 1;
                }
                Some('}') => {
                    self.i += 1;
                    return Ok(Json::Obj(map));
                }
                _ => return Err(format!("expected , or }} at {}", self.i)),
            }
        }
    }
}
