//! Meaning checks: builds a graph per process and reports the problems a syntax check cannot see.
//!
//! Codes: E1xx are errors, W2xx are warnings. (E0xx are syntax errors from the lexer and parser.)

use std::collections::{HashMap, VecDeque};

use crate::ast::{Item, Process};
use crate::diagnostic::{Diagnostic, Severity};
use crate::source::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Start,
    Task,
    Gateway,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub label: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessGraph {
    pub name: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Fewest steps from `start`; `None` for steps that cannot be reached.
    pub depth: Vec<Option<usize>>,
}

/// `complete` says the process had no syntax errors. The checks that need a whole, correct process
/// (missing start/end, missing next step, too few branches) run only then, and the whole-graph checks
/// (unreachable steps, endless loops) also need every name to resolve: after a syntax error or a typo
/// they would mostly report side effects of that one mistake, not real problems.
pub fn check(process: &Process, complete: bool, diags: &mut Vec<Diagnostic>) -> ProcessGraph {
    let name = process
        .name
        .as_ref()
        .map(|n| n.name.clone())
        .unwrap_or_default();
    let anchor = process.name.as_ref().map(|n| n.span).unwrap_or(Span::new(
        process.span.start,
        process.span.start + "process".len(),
    ));
    let mut nodes: Vec<Node> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut start_idx: Option<usize> = None;

    // Pass 1: declare every step, so a step may be used before the line that declares it.
    for item in &process.items {
        match item {
            Item::Start { span, .. } => {
                if start_idx.is_some() {
                    diags.push(
                        Diagnostic::new(
                            "E104",
                            Severity::Error,
                            "a process can have only one `start`".into(),
                            *span,
                        )
                        .with_hint("remove this line, or turn the extra entry point into a task"),
                    );
                } else {
                    start_idx = Some(nodes.len());
                    nodes.push(Node {
                        id: "start".into(),
                        kind: NodeKind::Start,
                        label: None,
                        span: *span,
                    });
                }
            }
            Item::End { id, label, .. } => declare(
                &mut nodes,
                &mut index,
                diags,
                id,
                NodeKind::End,
                label.as_ref().map(|l| l.value.clone()),
            ),
            Item::Task { id, label, .. } => declare(
                &mut nodes,
                &mut index,
                diags,
                id,
                NodeKind::Task,
                label.as_ref().map(|l| l.value.clone()),
            ),
            Item::Gateway { id, label, .. } => declare(
                &mut nodes,
                &mut index,
                diags,
                id,
                NodeKind::Gateway,
                label.as_ref().map(|l| l.value.clone()),
            ),
        }
    }
    if complete && start_idx.is_none() {
        diags.push(
            Diagnostic::new(
                "E103",
                Severity::Error,
                "the process has no `start`".into(),
                anchor,
            )
            .with_hint("add `start -> first_step` as the first line"),
        );
    }
    if complete && !nodes.iter().any(|n| n.kind == NodeKind::End) {
        diags.push(
            Diagnostic::new(
                "E105",
                Severity::Error,
                "the process has no `end`".into(),
                anchor,
            )
            .with_hint("add a line such as `end done`"),
        );
    }

    // Pass 2: edges and per-step rules.
    let mut edges: Vec<Edge> = Vec::new();
    let mut seen_start = false;
    let mut unresolved = false;
    for item in &process.items {
        match item {
            Item::Start { target, .. } => {
                if seen_start {
                    continue; // an extra `start` was already reported
                }
                seen_start = true;
                if let (Some(t), Some(from)) = (target, start_idx) {
                    unresolved |= !link(&nodes, &index, diags, &mut edges, from, t, None);
                }
            }
            Item::Task { id, next, .. } => {
                let Some(&from) = index.get(&id.name) else {
                    continue;
                };
                if nodes[from].span != id.span {
                    continue; // a duplicate declaration, already reported
                }
                match next {
                    Some(t) => {
                        unresolved |= !link(&nodes, &index, diags, &mut edges, from, t, None)
                    }
                    None if complete => {
                        unresolved = true; // an edge is missing, so graph-wide conclusions would be side effects
                        diags.push(
                            Diagnostic::new(
                                "E106",
                                Severity::Error,
                                format!("task `{}` does not say what comes next", id.name),
                                id.span,
                            )
                            .with_hint(format!("add `-> next_step`, or use `end {}`", id.name)),
                        );
                    }
                    None => {}
                }
            }
            Item::Gateway { id, branches, .. } => {
                let Some(&from) = index.get(&id.name) else {
                    continue;
                };
                if nodes[from].span != id.span {
                    continue;
                }
                if complete && branches.len() < 2 {
                    // With no branches at all, every step behind this gateway would look unreachable or endless.
                    unresolved |= branches.is_empty();
                    diags.push(
                        Diagnostic::new(
                            "E107",
                            Severity::Error,
                            format!("gateway `{}` needs at least two branches", id.name),
                            id.span,
                        )
                        .with_hint(
                            "a gateway chooses between paths, for example `yes -> a, no -> b`",
                        ),
                    );
                }
                let mut labels: HashMap<&str, ()> = HashMap::new();
                for b in branches {
                    if labels.insert(b.label.name.as_str(), ()).is_some() {
                        diags.push(Diagnostic::new(
                            "W202",
                            Severity::Warning,
                            format!(
                                "gateway `{}` has two branches called `{}`",
                                id.name, b.label.name
                            ),
                            b.label.span,
                        ));
                    }
                    if let Some(t) = &b.target {
                        unresolved |= !link(
                            &nodes,
                            &index,
                            diags,
                            &mut edges,
                            from,
                            t,
                            Some(b.label.name.clone()),
                        );
                    }
                }
            }
            Item::End { .. } => {}
        }
    }

    // Pass 3: reachability.
    let depth = bfs_depth(nodes.len(), &edges, start_idx);
    // Reachability needs the whole graph: with an unresolved name some edge is missing, and every step
    // behind it would be reported as unreachable or endless, a side effect of that one typo.
    if complete && !unresolved && start_idx.is_some() {
        for (i, n) in nodes.iter().enumerate() {
            if depth[i].is_none() {
                diags.push(
                    Diagnostic::new(
                        "W201",
                        Severity::Warning,
                        format!("`{}` can never be reached from `start`", n.id),
                        n.span,
                    )
                    .with_hint("point an earlier step at it, or remove it"),
                );
            }
        }
        let can_finish = reaches_end(&nodes, &edges);
        let has_end = nodes.iter().any(|n| n.kind == NodeKind::End);
        if has_end {
            for (i, n) in nodes.iter().enumerate() {
                if depth[i].is_some() && !can_finish[i] && n.kind != NodeKind::Start {
                    diags.push(Diagnostic::new(
                        "E108",
                        Severity::Error,
                        format!(
                            "`{}` can never reach an `end`: the process could run forever",
                            n.id
                        ),
                        n.span,
                    ));
                }
            }
        }
    }
    ProcessGraph {
        name,
        nodes,
        edges,
        depth,
    }
}

fn declare(
    nodes: &mut Vec<Node>,
    index: &mut HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
    id: &crate::ast::Ident,
    kind: NodeKind,
    label: Option<String>,
) {
    if index.contains_key(&id.name) {
        diags.push(
            Diagnostic::new(
                "E101",
                Severity::Error,
                format!("`{}` is already defined in this process", id.name),
                id.span,
            )
            .with_hint("every step needs its own name"),
        );
        return;
    }
    index.insert(id.name.clone(), nodes.len());
    nodes.push(Node {
        id: id.name.clone(),
        kind,
        label,
        span: id.span,
    });
}

fn link(
    nodes: &[Node],
    index: &HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
    edges: &mut Vec<Edge>,
    from: usize,
    target: &crate::ast::Ident,
    label: Option<String>,
) -> bool {
    match index.get(&target.name) {
        Some(&to) => {
            edges.push(Edge { from, to, label });
            true
        }
        None => {
            let mut d = Diagnostic::new(
                "E102",
                Severity::Error,
                format!("there is no step called `{}`", target.name),
                target.span,
            );
            if let Some(s) = suggest(
                &target.name,
                nodes
                    .iter()
                    .filter(|n| n.kind != NodeKind::Start)
                    .map(|n| n.id.as_str()),
            ) {
                d = d.with_hint(format!("did you mean `{s}`?"));
            }
            diags.push(d);
            false
        }
    }
}

fn bfs_depth(n: usize, edges: &[Edge], start: Option<usize>) -> Vec<Option<usize>> {
    let mut depth = vec![None; n];
    let Some(s) = start else { return depth };
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in edges {
        adj[e.from].push(e.to);
    }
    depth[s] = Some(0);
    let mut q = VecDeque::from([s]);
    while let Some(u) = q.pop_front() {
        for &v in &adj[u] {
            if depth[v].is_none() {
                depth[v] = Some(depth[u].unwrap_or(0) + 1);
                q.push_back(v);
            }
        }
    }
    depth
}

/// For every step: can some `end` be reached from it?
fn reaches_end(nodes: &[Node], edges: &[Edge]) -> Vec<bool> {
    let mut rev: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for e in edges {
        rev[e.to].push(e.from);
    }
    let mut ok = vec![false; nodes.len()];
    let mut q = VecDeque::new();
    for (i, n) in nodes.iter().enumerate() {
        if n.kind == NodeKind::End {
            ok[i] = true;
            q.push_back(i);
        }
    }
    while let Some(u) = q.pop_front() {
        for &p in &rev[u] {
            if !ok[p] {
                ok[p] = true;
                q.push_back(p);
            }
        }
    }
    ok
}

/// The closest known name, if it is close enough and unambiguous.
pub fn suggest<'a>(wanted: &str, known: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let wanted_len = wanted.chars().count();
    let limit = (wanted_len / 3).clamp(1, 2);
    let mut best: Option<(&str, usize)> = None;
    let mut tie = false;
    for k in known {
        // Names whose lengths differ by more than the limit cannot be within it: skip the comparison.
        let k_len = k.chars().count();
        if k_len > MAX_SUGGEST_LEN || k_len.abs_diff(wanted_len) > limit {
            continue;
        }
        let d = edit_distance(wanted, k);
        if d == 0 || d > limit {
            continue;
        }
        match best {
            Some((_, bd)) if d > bd => {}
            Some((b, bd)) if d == bd && b != k => tie = true,
            _ => {
                best = Some((k, d));
                tie = false;
            }
        }
    }
    if tie {
        None
    } else {
        best.map(|(k, _)| k)
    }
}

/// Names longer than this are never compared when suggesting a fix: a suggestion is for a typo in a
/// normal name, and the comparison cost grows with the product of the lengths.
const MAX_SUGGEST_LEN: usize = 64;

/// Edit distance where swapping two neighbouring letters counts as one edit (the most common typo),
/// i.e. the "optimal string alignment" variant of Damerau-Levenshtein. Keeps three rows, not the whole
/// table, so memory is linear in the shorter name.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev2: Vec<usize> = Vec::new();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut d = (prev[j - 1] + cost).min(prev[j] + 1).min(cur[j - 1] + 1);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d = d.min(prev2[j - 2] + 1);
            }
            cur.push(d);
        }
        prev2 = std::mem::replace(&mut prev, cur);
    }
    prev[b.len()]
}
