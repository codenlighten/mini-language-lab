use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub value: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub label: Ident,
    pub target: Option<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Start {
        target: Option<Ident>,
        span: Span,
    },
    End {
        id: Ident,
        label: Option<Label>,
        span: Span,
    },
    Task {
        id: Ident,
        label: Option<Label>,
        next: Option<Ident>,
        span: Span,
    },
    Gateway {
        id: Ident,
        label: Option<Label>,
        branches: Vec<Branch>,
        span: Span,
    },
}

impl Item {
    pub fn span(&self) -> Span {
        match self {
            Item::Start { span, .. }
            | Item::End { span, .. }
            | Item::Task { span, .. }
            | Item::Gateway { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub name: Option<Ident>,
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct File {
    pub processes: Vec<Process>,
}
