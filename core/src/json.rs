//! A tiny JSON writer, so the core crate needs no dependencies (and the WASM file stays small).

pub struct Json(pub String);

impl Json {
    pub fn new() -> Self {
        Json(String::new())
    }
    pub fn raw(&mut self, s: &str) -> &mut Self {
        self.0.push_str(s);
        self
    }
    pub fn str(&mut self, s: &str) -> &mut Self {
        self.0.push('"');
        for c in s.chars() {
            match c {
                '"' => self.0.push_str("\\\""),
                '\\' => self.0.push_str("\\\\"),
                '\n' => self.0.push_str("\\n"),
                '\r' => self.0.push_str("\\r"),
                '\t' => self.0.push_str("\\t"),
                c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                    self.0.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => self.0.push(c),
            }
        }
        self.0.push('"');
        self
    }
    pub fn num(&mut self, n: usize) -> &mut Self {
        self.0.push_str(&n.to_string());
        self
    }
    pub fn opt_str(&mut self, s: Option<&str>) -> &mut Self {
        match s {
            Some(s) => self.str(s),
            None => self.raw("null"),
        }
    }
}

impl Default for Json {
    fn default() -> Self {
        Self::new()
    }
}
