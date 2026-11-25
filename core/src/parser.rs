//! Recursive-descent parser with error recovery: one run reports every syntax error it can find,
//! not just the first. There is no nesting, so there is no recursion and no stack-depth risk.

use crate::ast::*;
use crate::diagnostic::{Diagnostic, Severity};
use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::Span;

pub fn parse(text: &str, tokens: &[Token], diags: &mut Vec<Diagnostic>) -> File {
    // Comments are trivia and `Error` tokens were already reported by the lexer.
    let toks: Vec<&Token> = tokens
        .iter()
        .filter(|t| !matches!(t.kind, TokenKind::Comment | TokenKind::Error))
        .collect();
    let mut p = Parser {
        text,
        toks,
        pos: 0,
        diags,
        eof: text.len(),
    };
    p.file()
}

struct Parser<'t, 'd> {
    text: &'t str,
    toks: Vec<&'t Token>,
    pos: usize,
    diags: &'d mut Vec<Diagnostic>,
    eof: usize,
}

impl Parser<'_, '_> {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos).copied()
    }
    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|t| &t.kind)
    }
    fn bump(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).map(|t| (*t).clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn here(&self) -> Span {
        match self.peek() {
            Some(t) => t.span,
            None => Span::new(self.eof, self.eof),
        }
    }
    fn err(&mut self, code: &'static str, msg: String, span: Span) {
        self.diags
            .push(Diagnostic::new(code, Severity::Error, msg, span));
    }
    fn describe(&self) -> String {
        match self.peek_kind() {
            None => "the end of the file".into(),
            Some(TokenKind::Keyword(_)) => "a keyword".into(),
            Some(TokenKind::Ident) => "a name".into(),
            Some(TokenKind::Str(_)) => "a string".into(),
            Some(TokenKind::Arrow) => "`->`".into(),
            Some(TokenKind::LBrace) => "`{`".into(),
            Some(TokenKind::RBrace) => "`}`".into(),
            Some(TokenKind::Comma) => "`,`".into(),
            Some(_) => "something else".into(),
        }
    }

    fn at_keyword(&self, k: Keyword) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Keyword(x)) if *x == k)
    }
    fn at_item_start(&self) -> bool {
        matches!(
            self.peek_kind(),
            Some(TokenKind::Keyword(
                Keyword::Start | Keyword::End | Keyword::Task | Keyword::Gateway
            ))
        )
    }

    fn expect_arrow(&mut self) -> bool {
        if matches!(self.peek_kind(), Some(TokenKind::Arrow)) {
            self.pos += 1;
            true
        } else {
            let (span, found) = (self.here(), self.describe());
            self.err("E011", format!("expected `->` but found {found}"), span);
            false
        }
    }

    /// Reads a name. A reserved word is reported but still used, so parsing can carry on.
    fn name(&mut self, what: &str) -> Option<Ident> {
        match self.peek_kind() {
            Some(TokenKind::Ident) => {
                let t = self.bump()?;
                Some(Ident {
                    name: self.text[t.span.start..t.span.end].to_string(),
                    span: t.span,
                })
            }
            Some(TokenKind::Keyword(_)) => {
                let t = self.bump()?;
                self.err(
                    "E013",
                    "a reserved word cannot be used as a name".into(),
                    t.span,
                );
                Some(Ident {
                    name: self.text[t.span.start..t.span.end].to_string(),
                    span: t.span,
                })
            }
            _ => {
                let (span, found) = (self.here(), self.describe());
                self.err("E010", format!("expected {what} but found {found}"), span);
                None
            }
        }
    }

    fn label(&mut self) -> Option<Label> {
        if let Some(TokenKind::Str(_)) = self.peek_kind() {
            let t = self.bump()?;
            if let TokenKind::Str(v) = t.kind {
                return Some(Label {
                    value: v,
                    span: t.span,
                });
            }
        }
        None
    }

    fn file(&mut self) -> File {
        let mut file = File::default();
        while self.peek().is_some() {
            if self.at_keyword(Keyword::Process) {
                file.processes.push(self.process());
            } else {
                let start = self.here();
                let mut last_end = start.end;
                while let Some(t) = self.peek() {
                    if self.at_keyword(Keyword::Process) {
                        break;
                    }
                    last_end = t.span.end;
                    self.pos += 1;
                }
                let span = Span::new(start.start, last_end);
                self.err("E012", "expected `process` at the top level".into(), span);
            }
        }
        file
    }

    fn process(&mut self) -> Process {
        let kw = self.bump().expect("caller checked for the keyword").span;
        let name = self.name("a process name");
        let mut end = name.as_ref().map(|n| n.span.end).unwrap_or(kw.end);
        let mut items = Vec::new();
        if matches!(self.peek_kind(), Some(TokenKind::LBrace)) {
            self.pos += 1;
        } else {
            let (span, found) = (self.here(), self.describe());
            self.err(
                "E011",
                format!("expected `{{` after the process name but found {found}"),
                span,
            );
        }
        loop {
            match self.peek_kind() {
                None => {
                    let span = Span::new(self.eof, self.eof);
                    self.err("E015", "missing `}` at the end of the process".into(), span);
                    break;
                }
                Some(TokenKind::RBrace) => {
                    end = self.bump().map(|t| t.span.end).unwrap_or(end);
                    break;
                }
                Some(TokenKind::Keyword(Keyword::Process)) => {
                    let span = self.here();
                    self.err("E015", "missing `}` before the next process".into(), span);
                    break;
                }
                _ if self.at_item_start() => {
                    if let Some(item) = self.item() {
                        end = end.max(item.span().end);
                        items.push(item);
                    }
                }
                _ => {
                    let span = self.here();
                    let found = self.describe();
                    self.err(
                        "E012",
                        format!(
                            "expected `start`, `task`, `gateway`, `end` or `}}` but found {found}"
                        ),
                        span,
                    );
                    self.recover();
                }
            }
        }
        Process {
            name,
            items,
            span: Span::new(kw.start, end.max(kw.end)),
        }
    }

    /// Skips to the next token that can begin an item or close the process.
    fn recover(&mut self) {
        while let Some(k) = self.peek_kind() {
            if matches!(k, TokenKind::RBrace | TokenKind::Keyword(_)) {
                break;
            }
            self.pos += 1;
        }
    }

    fn item(&mut self) -> Option<Item> {
        let kw_tok = self.bump()?;
        let TokenKind::Keyword(kw) = kw_tok.kind else {
            return None;
        };
        let begin = kw_tok.span.start;
        match kw {
            Keyword::Start => {
                let mut target = None;
                let mut end = kw_tok.span.end;
                if self.expect_arrow() {
                    end = self
                        .toks
                        .get(self.pos.wrapping_sub(1))
                        .map(|t| t.span.end)
                        .unwrap_or(end);
                    target = self.name("the first step after `start ->`");
                    if let Some(t) = &target {
                        end = t.span.end;
                    }
                }
                Some(Item::Start {
                    target,
                    span: Span::new(begin, end),
                })
            }
            Keyword::End => {
                let id = self.name("a name for the end")?;
                let label = self.label();
                let end = label.as_ref().map(|l| l.span.end).unwrap_or(id.span.end);
                Some(Item::End {
                    id,
                    label,
                    span: Span::new(begin, end),
                })
            }
            Keyword::Task => {
                let id = self.name("a task name")?;
                let label = self.label();
                let mut end = label.as_ref().map(|l| l.span.end).unwrap_or(id.span.end);
                let mut next = None;
                if matches!(self.peek_kind(), Some(TokenKind::Arrow)) {
                    self.pos += 1;
                    next = self.name("the next step after `->`");
                    if let Some(n) = &next {
                        end = n.span.end;
                    }
                }
                Some(Item::Task {
                    id,
                    label,
                    next,
                    span: Span::new(begin, end),
                })
            }
            Keyword::Gateway => {
                let id = self.name("a gateway name")?;
                let label = self.label();
                let mut end = label.as_ref().map(|l| l.span.end).unwrap_or(id.span.end);
                let mut branches = Vec::new();
                if matches!(self.peek_kind(), Some(TokenKind::LBrace)) {
                    self.pos += 1;
                } else {
                    let (span, found) = (self.here(), self.describe());
                    self.err(
                        "E011",
                        format!("expected `{{` to start the gateway's branches but found {found}"),
                        span,
                    );
                    return Some(Item::Gateway {
                        id,
                        label,
                        branches,
                        span: Span::new(begin, end),
                    });
                }
                loop {
                    match self.peek_kind() {
                        Some(TokenKind::RBrace) => {
                            end = self.bump().map(|t| t.span.end).unwrap_or(end);
                            break;
                        }
                        None | Some(TokenKind::Keyword(_)) => {
                            let span = self.here();
                            self.err("E015", "missing `}` at the end of the gateway".into(), span);
                            break;
                        }
                        _ => {}
                    }
                    let Some(lab) = self.name("a branch label such as `yes`") else {
                        self.recover();
                        continue;
                    };
                    let lab_start = lab.span.start;
                    let mut target = None;
                    let mut bend = lab.span.end;
                    if self.expect_arrow() {
                        target = self.name("the step this branch leads to");
                        if let Some(t) = &target {
                            bend = t.span.end;
                        }
                    }
                    branches.push(Branch {
                        label: lab,
                        target,
                        span: Span::new(lab_start, bend),
                    });
                    match self.peek_kind() {
                        Some(TokenKind::Comma) => self.pos += 1,
                        Some(TokenKind::RBrace) => {}
                        _ => {
                            let (span, found) = (self.here(), self.describe());
                            self.err(
                                "E014",
                                format!("expected `,` or `}}` after a branch but found {found}"),
                                span,
                            );
                            self.recover();
                        }
                    }
                }
                Some(Item::Gateway {
                    id,
                    label,
                    branches,
                    span: Span::new(begin, end),
                })
            }
            Keyword::Process => None,
        }
    }
}
