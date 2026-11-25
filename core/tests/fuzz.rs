//! Deterministic fuzzing: thousands of random and mutated programs must never panic, and every
//! span must be in bounds, on a character boundary, and map to a consistent UTF-16 range.
mod common;
use common::*;
use procflow_core::ast::Item;
use procflow_core::lexer::TokenKind;
use procflow_core::source::Source;
use procflow_core::{analysis_json, analyze, format};

const FRAGMENTS: &[&str] = &[
    "process ",
    "start ",
    "task ",
    "gateway ",
    "end ",
    "->",
    "{",
    "}",
    ",",
    "\"",
    "\\",
    "\\\"",
    "//",
    "\n",
    " ",
    "\t",
    "\r\n",
    "a",
    "b",
    "node_1",
    "yes",
    "no",
    "é",
    "😀",
    "\u{2028}",
    "日本",
    "\0",
    "\"label\"",
    "x -> y",
    "/*",
    "*/",
    "-",
    ">",
    "9",
];

fn random_text(rng: &mut Rng) -> String {
    let n = 1 + rng.below(40);
    (0..n)
        .map(|_| FRAGMENTS[rng.below(FRAGMENTS.len())])
        .collect()
}

fn boundaries(s: &str) -> Vec<usize> {
    s.char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(s.len()))
        .collect()
}

fn mutate(rng: &mut Rng, base: &str) -> String {
    let mut s = base.to_string();
    for _ in 0..1 + rng.below(4) {
        let b = boundaries(&s);
        let i = b[rng.below(b.len())];
        let j = b[rng.below(b.len())];
        let (lo, hi) = (i.min(j), i.max(j));
        match rng.below(4) {
            0 => s.replace_range(lo..hi, ""),
            1 => s.insert_str(lo, FRAGMENTS[rng.below(FRAGMENTS.len())]),
            2 => {
                let piece = s[lo..hi].to_string();
                s.insert_str(hi, &piece);
            }
            _ => s.replace_range(lo..hi, FRAGMENTS[rng.below(FRAGMENTS.len())]),
        }
    }
    s
}

fn check(text: &str) {
    let a = analyze(text);
    let b = boundaries(text);
    let src = Source::new(text);
    let u16len = text.encode_utf16().count();
    let mut spans: Vec<(usize, usize)> = a
        .tokens
        .iter()
        .map(|t| (t.span.start, t.span.end))
        .collect();
    spans.extend(a.diagnostics.iter().map(|d| (d.span.start, d.span.end)));
    for (s, e) in spans {
        assert!(
            s <= e && e <= text.len(),
            "span out of bounds: {s}..{e} in {text:?}"
        );
        assert!(
            b.contains(&s) && b.contains(&e),
            "span not on a char boundary: {s}..{e} in {text:?}"
        );
        let (ps, pe) = (src.position(s), src.position(e));
        assert!(ps.offset <= pe.offset && pe.offset <= u16len);
        let u: Vec<u16> = text.encode_utf16().collect();
        assert_eq!(
            String::from_utf16(&u[ps.offset..pe.offset]).unwrap(),
            &text[s..e]
        );
    }
    // Tokens never overlap and are in order.
    for w in a.tokens.windows(2) {
        assert!(
            w[0].span.end <= w[1].span.start,
            "tokens overlap in {text:?}"
        );
    }
    // Diagnostics are sorted.
    for w in a.diagnostics.windows(2) {
        assert!(w[0].span.start <= w[1].span.start);
    }
    assert!(
        is_valid_json(&analysis_json(text, &a)),
        "invalid JSON for {text:?}"
    );
    // Analysis is deterministic.
    assert_eq!(a.diagnostics, analyze(text).diagnostics);
    // If it formats, the result is stable, keeps the structure and every comment.
    if let Some(out) = format(text) {
        let again = format(&out)
            .unwrap_or_else(|| panic!("formatted output no longer parses: {out:?} from {text:?}"));
        assert_eq!(out, again, "formatting is not idempotent for {text:?}");
        let (a2, a1) = (analyze(&out), analyze(text));
        assert_eq!(
            item_count(&a1),
            item_count(&a2),
            "items changed for {text:?}"
        );
        let cc = |x: &procflow_core::Analysis| {
            x.tokens
                .iter()
                .filter(|t| t.kind == TokenKind::Comment)
                .count()
        };
        assert_eq!(cc(&a1), cc(&a2), "comments lost for {text:?} -> {out:?}");
    }
}

fn item_count(a: &procflow_core::Analysis) -> Vec<(usize, usize)> {
    a.file
        .processes
        .iter()
        .map(|p| {
            let branches: usize = p
                .items
                .iter()
                .map(|i| {
                    if let Item::Gateway { branches, .. } = i {
                        branches.len()
                    } else {
                        0
                    }
                })
                .sum();
            (p.items.len(), branches)
        })
        .collect()
}

#[test]
fn random_fragment_soup_never_breaks_an_invariant() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for _ in 0..6000 {
        check(&random_text(&mut rng));
    }
}

#[test]
fn mutated_valid_programs_never_break_an_invariant() {
    let mut rng = Rng(0xDEADBEEFCAFEF00D);
    for _ in 0..6000 {
        check(&mutate(&mut rng, SAMPLE));
    }
}

#[test]
fn every_prefix_of_the_sample_is_handled() {
    for end in boundaries(SAMPLE) {
        check(&SAMPLE[..end]);
    }
}

#[test]
fn every_single_character_deletion_of_the_sample_is_handled() {
    let b = boundaries(SAMPLE);
    for w in b.windows(2) {
        let mut s = SAMPLE.to_string();
        s.replace_range(w[0]..w[1], "");
        check(&s);
    }
}
