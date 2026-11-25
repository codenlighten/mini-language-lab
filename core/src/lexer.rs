//! Turns source text into tokens. Never fails: bad input becomes `Error` tokens plus diagnostics.

use crate::diagnostic::{Diagnostic, Severity};
use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Process,
    Start,
    End,
    Task,
    Gateway,
}

impl Keyword {
    pub const ALL: [(&'static str, Keyword); 5] = [
        ("process", Keyword::Process),
        ("start", Keyword::Start),
        ("end", Keyword::End),
        ("task", Keyword::Task),
        ("gateway", Keyword::Gateway),
    ];
    pub fn from_word(word: &str) -> Option<Keyword> {
        Self::ALL.iter().find(|(w, _)| *w == word).map(|(_, k)| *k)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword(Keyword),
    Ident,
    /// A string literal; the unescaped value is stored.
    Str(String),
    Arrow,
    LBrace,
    RBrace,
    Comma,
    Comment,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub const MAX_SOURCE_BYTES: usize = 1 << 20;

pub fn lex(text: &str, diags: &mut Vec<Diagnostic>) -> Vec<Token> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let start = i;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    span: Span::new(start, i),
                });
            }
            b'-' if bytes.get(i + 1) == Some(&b'>') => {
                tokens.push(Token {
                    kind: TokenKind::Arrow,
                    span: Span::new(i, i + 2),
                });
                i += 2;
            }
            b'{' => {
                tokens.push(Token {
                    kind: TokenKind::LBrace,
                    span: Span::new(i, i + 1),
                });
                i += 1;
            }
            b'}' => {
                tokens.push(Token {
                    kind: TokenKind::RBrace,
                    span: Span::new(i, i + 1),
                });
                i += 1;
            }
            b',' => {
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    span: Span::new(i, i + 1),
                });
                i += 1;
            }
            b'"' => i = lex_string(text, i, &mut tokens, diags),
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let kind = match Keyword::from_word(&text[start..i]) {
                    Some(k) => TokenKind::Keyword(k),
                    None => TokenKind::Ident,
                };
                tokens.push(Token {
                    kind,
                    span: Span::new(start, i),
                });
            }
            _ => {
                // A run of stray characters is one problem, not one per character.
                let start = i;
                while i < bytes.len() && !ends_stray_run(bytes, i) {
                    i += text[i..].chars().next().map_or(1, char::len_utf8);
                }
                let span = Span::new(start, i);
                let run = &text[start..i];
                let msg = match run.chars().next() {
                    Some(ch) if run.chars().count() == 1 && ch.is_alphabetic() => {
                        format!("`{ch}` is not allowed in a name: names use only ASCII letters, digits and `_`")
                    }
                    Some(ch) if run.chars().count() == 1 => format!("unexpected character `{ch}`"),
                    _ => format!("unexpected characters `{run}`"),
                };
                diags.push(Diagnostic::new("E001", Severity::Error, msg, span));
                tokens.push(Token {
                    kind: TokenKind::Error,
                    span,
                });
            }
        }
    }
    tokens
}

/// Lexes a string literal starting at the opening quote. Returns the index after it.
fn lex_string(
    text: &str,
    quote: usize,
    tokens: &mut Vec<Token>,
    diags: &mut Vec<Diagnostic>,
) -> usize {
    let mut value = String::new();
    let mut chars = text[quote + 1..].char_indices();
    while let Some((off, ch)) = chars.next() {
        let at = quote + 1 + off;
        match ch {
            '"' => {
                let end = at + 1;
                tokens.push(Token {
                    kind: TokenKind::Str(value),
                    span: Span::new(quote, end),
                });
                return end;
            }
            '\n' => {
                let span = Span::new(quote, at);
                diags.push(Diagnostic::new(
                    "E002",
                    Severity::Error,
                    "string is not closed before the end of the line".into(),
                    span,
                ));
                tokens.push(Token {
                    kind: TokenKind::Error,
                    span,
                });
                return at;
            }
            '\\' => match chars.next() {
                Some((_, '"')) => value.push('"'),
                Some((_, '\\')) => value.push('\\'),
                Some((_, 'n')) => value.push('\n'),
                Some((o2, other)) => {
                    let bad = quote + 1 + o2;
                    diags.push(Diagnostic::new(
                        "E003",
                        Severity::Error,
                        format!("unknown escape `\\{other}`: use \\\" \\\\ or \\n"),
                        Span::new(at, bad + other.len_utf8()),
                    ));
                    value.push(other);
                }
                None => break,
            },
            other => value.push(other),
        }
    }
    let span = Span::new(quote, text.len());
    diags.push(Diagnostic::new(
        "E002",
        Severity::Error,
        "string is not closed before the end of the file".into(),
        span,
    ));
    tokens.push(Token {
        kind: TokenKind::Error,
        span,
    });
    text.len()
}

/// True when the byte at `i` can start something the lexer understands (or is whitespace), which
/// ends a run of stray characters. Multi-byte characters never end a run: they are always stray.
fn ends_stray_run(bytes: &[u8], i: usize) -> bool {
    let c = bytes[i];
    match c {
        b' ' | b'\t' | b'\r' | b'\n' | b'{' | b'}' | b',' | b'"' => true,
        b'-' => bytes.get(i + 1) == Some(&b'>'),
        b'/' => bytes.get(i + 1) == Some(&b'/'),
        c => c.is_ascii_alphabetic() || c == b'_',
    }
}
