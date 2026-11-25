//! Canonical formatter. Refuses to format text with syntax errors, and never drops a comment.
//!
//! Comment placement: a comment on its own line stays above the item that follows it; a comment at
//! the end of an item's line stays on that line; a comment written inside a gateway's braces is moved
//! to just below the gateway. Blank lines are not preserved, except one between processes.

use crate::ast::*;
use crate::lexer::{Token, TokenKind};

pub fn format(text: &str, tokens: &[Token], file: &File) -> String {
    let comments: Vec<(usize, usize)> = tokens
        .iter()
        .filter(|t| t.kind == TokenKind::Comment)
        .map(|t| (t.span.start, t.span.end))
        .collect();
    let mut out = Out {
        text,
        comments: &comments,
        next: 0,
        buf: String::new(),
    };
    for (i, p) in file.processes.iter().enumerate() {
        if i > 0 {
            out.buf.push('\n');
        }
        out.process(p);
    }
    out.leading(usize::MAX, 0);
    out.buf
}

struct Out<'a> {
    text: &'a str,
    comments: &'a [(usize, usize)],
    next: usize,
    buf: String,
}

impl Out<'_> {
    fn line(&mut self, indent: usize, s: &str) {
        for _ in 0..indent {
            self.buf.push_str("  ");
        }
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    fn comment_text(&self, (s, e): (usize, usize)) -> &str {
        self.text[s..e].trim_end()
    }

    /// Emits every not-yet-written comment that starts before `pos`, each on its own line.
    fn leading(&mut self, pos: usize, indent: usize) {
        while let Some(&c) = self.comments.get(self.next) {
            if c.0 >= pos {
                break;
            }
            let t = self.comment_text(c).to_string();
            self.line(indent, &t);
            self.next += 1;
        }
    }

    /// Writes one item. `lines` are its already-formatted lines; a comment that starts on the same
    /// source line right after the item is kept at the end of the item's last line.
    fn item(&mut self, span: crate::source::Span, mut lines: Vec<String>, indent: usize) {
        self.leading(span.start, indent);
        // Comments that begin inside the item (between gateway braces) cannot stay there.
        let mut inner = Vec::new();
        while let Some(&c) = self.comments.get(self.next) {
            if c.0 < span.end {
                inner.push(c);
                self.next += 1;
            } else {
                break;
            }
        }
        if let Some(&c) = self.comments.get(self.next) {
            // Trailing only if nothing but spaces lies between the item and the comment; otherwise the comment
            // belongs to whatever comes next on the line (another item, or a brace).
            if self.text[span.end..c.0]
                .chars()
                .all(|ch| matches!(ch, ' ' | '\t' | '\r'))
            {
                let t = self.comment_text(c).to_string();
                if let Some(last) = lines.last_mut() {
                    last.push_str("  ");
                    last.push_str(&t);
                }
                self.next += 1;
            }
        }
        for l in lines {
            self.line(indent, &l);
        }
        for c in inner {
            let t = self.comment_text(c).to_string();
            self.line(indent, &t);
        }
    }

    fn process(&mut self, p: &Process) {
        self.leading(p.span.start, 0);
        let name = p.name.as_ref().map(|n| n.name.as_str()).unwrap_or("");
        self.line(0, &format!("process {name} {{"));
        for it in &p.items {
            match it {
                Item::Start { target, span } => {
                    let t = target.as_ref().map(|t| t.name.as_str()).unwrap_or("");
                    self.item(*span, vec![format!("start -> {t}")], 1);
                }
                Item::End { id, label, span } => {
                    self.item(*span, vec![format!("end {}{}", id.name, lab(label))], 1);
                }
                Item::Task {
                    id,
                    label,
                    next,
                    span,
                } => {
                    let tail = next
                        .as_ref()
                        .map(|n| format!(" -> {}", n.name))
                        .unwrap_or_default();
                    self.item(
                        *span,
                        vec![format!("task {}{}{tail}", id.name, lab(label))],
                        1,
                    );
                }
                Item::Gateway {
                    id,
                    label,
                    branches,
                    span,
                } => {
                    let head = format!("gateway {}{}", id.name, lab(label));
                    let lines = if branches.is_empty() {
                        vec![format!("{head} {{}}")]
                    } else {
                        let mut v = vec![format!("{head} {{")];
                        for (i, b) in branches.iter().enumerate() {
                            let t = b.target.as_ref().map(|t| t.name.as_str()).unwrap_or("");
                            let comma = if i + 1 < branches.len() { "," } else { "" };
                            v.push(format!("  {} -> {t}{comma}", b.label.name));
                        }
                        v.push("}".into());
                        v
                    };
                    self.item(*span, lines, 1);
                }
            }
        }
        // Comments written before the closing brace stay inside the process.
        self.leading(p.span.end.saturating_sub(1), 1);
        self.line(0, "}");
    }
}

fn lab(l: &Option<Label>) -> String {
    match l {
        Some(l) => format!(" \"{}\"", escape(&l.value)),
        None => String::new(),
    }
}

fn escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c => o.push(c),
        }
    }
    o
}
