#![allow(dead_code)]
use procflow_core::analyze;

pub const SAMPLE: &str = r#"// Order handling
process Order {
  start -> check
  task check "Check stock" -> decide
  gateway decide "In stock?" {
    yes -> ship,
    no -> backorder
  }
  task ship "Ship the order" -> done
  task backorder "Order from supplier" -> check
  end done "Order complete"
}
"#;

pub fn codes(src: &str) -> Vec<&'static str> {
    analyze(src).diagnostics.iter().map(|d| d.code).collect()
}

pub fn has(src: &str, code: &str) -> bool {
    codes(src).contains(&code)
}

/// Wraps steps in a process with a valid start and end so a test can focus on one rule.
pub fn proc(body: &str) -> String {
    format!("process P {{\n{body}\n}}\n")
}

/// A small deterministic random generator (xorshift64*), so fuzz failures can be replayed.
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// A minimal JSON validator (objects, arrays, strings, numbers, true/false/null), enough to prove the
/// output is well-formed without any dependency.
pub fn is_valid_json(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    fn ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && matches!(b[*i], b' ' | b'\n' | b'\r' | b'\t') {
            *i += 1;
        }
    }
    fn value(b: &[u8], i: &mut usize, depth: usize) -> bool {
        if depth > 64 {
            return false;
        }
        ws(b, i);
        match b.get(*i) {
            Some(b'{') => {
                *i += 1;
                ws(b, i);
                if b.get(*i) == Some(&b'}') {
                    *i += 1;
                    return true;
                }
                loop {
                    ws(b, i);
                    if !string(b, i) {
                        return false;
                    }
                    ws(b, i);
                    if b.get(*i) != Some(&b':') {
                        return false;
                    }
                    *i += 1;
                    if !value(b, i, depth + 1) {
                        return false;
                    }
                    ws(b, i);
                    match b.get(*i) {
                        Some(b',') => *i += 1,
                        Some(b'}') => {
                            *i += 1;
                            return true;
                        }
                        _ => return false,
                    }
                }
            }
            Some(b'[') => {
                *i += 1;
                ws(b, i);
                if b.get(*i) == Some(&b']') {
                    *i += 1;
                    return true;
                }
                loop {
                    if !value(b, i, depth + 1) {
                        return false;
                    }
                    ws(b, i);
                    match b.get(*i) {
                        Some(b',') => *i += 1,
                        Some(b']') => {
                            *i += 1;
                            return true;
                        }
                        _ => return false,
                    }
                }
            }
            Some(b'"') => string(b, i),
            Some(b't') => lit(b, i, b"true"),
            Some(b'f') => lit(b, i, b"false"),
            Some(b'n') => lit(b, i, b"null"),
            Some(c) if c.is_ascii_digit() => {
                while *i < b.len() && b[*i].is_ascii_digit() {
                    *i += 1;
                }
                true
            }
            _ => false,
        }
    }
    fn lit(b: &[u8], i: &mut usize, w: &[u8]) -> bool {
        if b[*i..].starts_with(w) {
            *i += w.len();
            true
        } else {
            false
        }
    }
    fn string(b: &[u8], i: &mut usize) -> bool {
        if b.get(*i) != Some(&b'"') {
            return false;
        }
        *i += 1;
        while *i < b.len() {
            match b[*i] {
                b'"' => {
                    *i += 1;
                    return true;
                }
                b'\\' => match b.get(*i + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => *i += 2,
                    Some(b'u') => {
                        if b.len() < *i + 6
                            || !b[*i + 2..*i + 6].iter().all(|c| c.is_ascii_hexdigit())
                        {
                            return false;
                        }
                        *i += 6
                    }
                    _ => return false,
                },
                c if c < 0x20 => return false,
                _ => *i += 1,
            }
        }
        false
    }
    if !value(b, &mut i, 0) {
        return false;
    }
    ws(b, &mut i);
    i == b.len()
}
