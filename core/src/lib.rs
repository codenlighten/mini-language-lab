//! Procflow: a small process-description language with the tooling an editor needs.
//!
//! ```
//! let a = procflow_core::analyze("process P { start -> a  task a -> done  end done }");
//! assert!(a.diagnostics.is_empty());
//! ```

pub mod ast;
pub mod diagnostic;
mod format;
mod json;
pub mod lexer;
pub mod parser;
pub mod source;
pub mod validate;

use std::collections::HashSet;

use diagnostic::{Diagnostic, Severity};
use json::Json;
use lexer::{Token, TokenKind};
use source::{Source, Span};
use validate::{NodeKind, ProcessGraph};

#[derive(Debug, Clone)]
pub struct Analysis {
    pub tokens: Vec<Token>,
    pub file: ast::File,
    /// Sorted by position, then code.
    pub diagnostics: Vec<Diagnostic>,
    pub graphs: Vec<ProcessGraph>,
}

impl Analysis {
    pub fn has_syntax_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.code.starts_with("E0") || d.code == "E900")
    }
}

/// Lexes, parses and checks `text`. Never panics, whatever the input.
pub fn analyze(text: &str) -> Analysis {
    let mut diagnostics = Vec::new();
    if text.len() > lexer::MAX_SOURCE_BYTES {
        diagnostics.push(Diagnostic::new(
            "E900",
            Severity::Error,
            format!(
                "the source is larger than {} bytes, which this tool does not analyze",
                lexer::MAX_SOURCE_BYTES
            ),
            Span::new(0, 0),
        ));
        return Analysis {
            tokens: Vec::new(),
            file: ast::File::default(),
            diagnostics,
            graphs: Vec::new(),
        };
    }
    let tokens = lexer::lex(text, &mut diagnostics);
    let file = parser::parse(text, &tokens, &mut diagnostics);
    let mut graphs = Vec::new();
    let mut names: HashSet<&str> = HashSet::new();
    // A process is "complete" when no syntax error falls between its start and the next process's start.
    let syntax_error_starts: Vec<usize> = diagnostics
        .iter()
        .filter(|d| d.code.starts_with("E0"))
        .map(|d| d.span.start)
        .collect();
    for (i, p) in file.processes.iter().enumerate() {
        let to = file
            .processes
            .get(i + 1)
            .map_or(usize::MAX, |n| n.span.start);
        let complete = !syntax_error_starts
            .iter()
            .any(|&s| s >= p.span.start && s < to);
        if let Some(n) = &p.name {
            if !names.insert(n.name.as_str()) {
                diagnostics.push(Diagnostic::new(
                    "E110",
                    Severity::Error,
                    format!("a process called `{}` already exists", n.name),
                    n.span,
                ));
            }
        }
        graphs.push(validate::check(p, complete, &mut diagnostics));
    }
    diagnostics.sort_by(|a, b| {
        (a.span.start, a.span.end, a.code).cmp(&(b.span.start, b.span.end, b.code))
    });
    Analysis {
        tokens,
        file,
        diagnostics,
        graphs,
    }
}

/// The canonical form of `text`, or `None` if it has syntax errors.
pub fn format(text: &str) -> Option<String> {
    let a = analyze(text);
    if a.has_syntax_errors() {
        return None;
    }
    Some(format::format(text, &a.tokens, &a.file))
}

fn token_class(k: &TokenKind) -> &'static str {
    match k {
        TokenKind::Keyword(_) => "keyword",
        TokenKind::Ident => "name",
        TokenKind::Str(_) => "string",
        TokenKind::Arrow | TokenKind::LBrace | TokenKind::RBrace | TokenKind::Comma => "punct",
        TokenKind::Comment => "comment",
        TokenKind::Error => "error",
    }
}

fn kind_name(k: NodeKind) -> &'static str {
    match k {
        NodeKind::Start => "start",
        NodeKind::Task => "task",
        NodeKind::Gateway => "gateway",
        NodeKind::End => "end",
    }
}

/// The result as JSON for a JavaScript editor. All offsets are UTF-16 code units, lines and columns
/// are zero-based.
pub fn analysis_json(text: &str, a: &Analysis) -> String {
    let src = Source::new(text);
    let mut j = Json::new();
    j.raw("{\"tokens\":[");
    for (i, t) in a.tokens.iter().enumerate() {
        if i > 0 {
            j.raw(",");
        }
        let (s, e) = (src.position(t.span.start), src.position(t.span.end));
        j.raw("{\"kind\":")
            .str(token_class(&t.kind))
            .raw(",\"start\":")
            .num(s.offset)
            .raw(",\"end\":")
            .num(e.offset)
            .raw("}");
    }
    j.raw("],\"diagnostics\":[");
    for (i, d) in a.diagnostics.iter().enumerate() {
        if i > 0 {
            j.raw(",");
        }
        let (s, e) = (src.position(d.span.start), src.position(d.span.end));
        j.raw("{\"code\":").str(d.code);
        j.raw(",\"severity\":")
            .str(if d.severity == Severity::Error {
                "error"
            } else {
                "warning"
            });
        j.raw(",\"message\":").str(&d.message);
        j.raw(",\"hint\":").opt_str(d.hint.as_deref());
        j.raw(",\"start\":")
            .num(s.offset)
            .raw(",\"end\":")
            .num(e.offset);
        j.raw(",\"line\":")
            .num(s.line)
            .raw(",\"column\":")
            .num(s.column);
        j.raw(",\"endLine\":")
            .num(e.line)
            .raw(",\"endColumn\":")
            .num(e.column)
            .raw("}");
    }
    j.raw("],\"processes\":[");
    for (i, g) in a.graphs.iter().enumerate() {
        if i > 0 {
            j.raw(",");
        }
        j.raw("{\"name\":").str(&g.name).raw(",\"nodes\":[");
        for (k, n) in g.nodes.iter().enumerate() {
            if k > 0 {
                j.raw(",");
            }
            j.raw("{\"id\":")
                .str(&n.id)
                .raw(",\"kind\":")
                .str(kind_name(n.kind));
            j.raw(",\"label\":").opt_str(n.label.as_deref());
            match g.depth[k] {
                Some(d) => j.raw(",\"depth\":").num(d),
                None => j.raw(",\"depth\":null"),
            };
            j.raw("}");
        }
        j.raw("],\"edges\":[");
        for (k, e) in g.edges.iter().enumerate() {
            if k > 0 {
                j.raw(",");
            }
            j.raw("{\"from\":")
                .str(&g.nodes[e.from].id)
                .raw(",\"to\":")
                .str(&g.nodes[e.to].id);
            j.raw(",\"label\":").opt_str(e.label.as_deref()).raw("}");
        }
        j.raw("]}");
    }
    j.raw("]}");
    j.0
}
