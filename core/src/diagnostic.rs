use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable code such as `E102`; see the table in the README.
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    /// Optional advice, for example a "did you mean" suggestion.
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn new(code: &'static str, severity: Severity, message: String, span: Span) -> Self {
        Diagnostic {
            code,
            severity,
            message,
            span,
            hint: None,
        }
    }
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}
