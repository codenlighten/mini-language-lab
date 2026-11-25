mod common;
use common::*;
use procflow_core::analyze;
use procflow_core::lexer::{lex, Keyword, TokenKind};

fn kinds(src: &str) -> Vec<TokenKind> {
    lex(src, &mut Vec::new())
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

#[test]
fn the_sample_program_has_no_diagnostics() {
    assert_eq!(codes(SAMPLE), Vec::<&str>::new());
}

#[test]
fn lexer_recognises_every_token_kind() {
    let k = kinds("process P { start -> a, // hi\n task \"x\\\"y\" }");
    assert_eq!(
        k,
        vec![
            TokenKind::Keyword(Keyword::Process),
            TokenKind::Ident,
            TokenKind::LBrace,
            TokenKind::Keyword(Keyword::Start),
            TokenKind::Arrow,
            TokenKind::Ident,
            TokenKind::Comma,
            TokenKind::Comment,
            TokenKind::Keyword(Keyword::Task),
            TokenKind::Str("x\"y".into()),
            TokenKind::RBrace,
        ]
    );
}

#[test]
fn keywords_are_whole_words_only() {
    assert_eq!(
        kinds("starting endless"),
        vec![TokenKind::Ident, TokenKind::Ident]
    );
}

#[test]
fn crlf_line_endings_are_accepted() {
    assert_eq!(codes(&SAMPLE.replace('\n', "\r\n")), Vec::<&str>::new());
}

#[test]
fn unterminated_string_is_reported_once_per_string() {
    // Only the string problem is reported: the half-parsed task does not also produce "no next step"
    // and "unreachable" follow-on errors.
    assert_eq!(
        codes(&proc("start -> a\ntask a \"oops -> b\nend b")),
        vec!["E002"]
    );
    assert!(has("process P { task a \"never closed", "E002"));
}

#[test]
fn unknown_escape_is_an_error_with_the_escape_in_the_message() {
    let a = analyze(&proc("start -> a\ntask a \"x\\qy\" -> b\nend b"));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "E003")
        .expect("E003");
    assert!(d.message.contains("\\q"));
}

#[test]
fn non_ascii_names_are_refused_with_an_explanation() {
    let a = analyze(&proc("start -> a\ntask café -> b\nend b"));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "E001")
        .expect("E001");
    assert!(d.message.contains("ASCII"), "{}", d.message);
}

#[test]
fn stray_characters_are_reported_not_fatal() {
    assert!(has(&proc("start -> a\n@ task a -> b\nend b"), "E001"));
}

#[test]
fn many_syntax_errors_are_reported_in_one_run() {
    let src = proc("start a\ntask -> b\nbogus line\nend b\ngateway g { yes -> b no -> b }");
    let c = codes(&src);
    assert!(c.contains(&"E011"), "{c:?}"); // `start a` misses the arrow
    assert!(c.contains(&"E010"), "{c:?}"); // `task ->` misses a name
    assert!(c.contains(&"E012"), "{c:?}"); // `bogus line`
    assert!(c.contains(&"E014"), "{c:?}"); // missing comma between branches
}

#[test]
fn recovery_keeps_later_items_so_their_problems_are_still_found() {
    // The first line is broken; the second still has a real undefined-target error to report.
    let src = proc("start -> a\ntask 123\ntask a -> missing\nend z");
    let c = codes(&src);
    assert!(c.contains(&"E102"), "{c:?}");
}

#[test]
fn reserved_words_cannot_be_names() {
    assert!(has(
        &proc("start -> end\ntask end -> end2\nend end2"),
        "E013"
    ));
}

#[test]
fn missing_closing_brace_does_not_swallow_the_next_process() {
    let a = analyze("process A {\n start -> x\n end x\nprocess B { start -> y\n end y }");
    assert!(a.diagnostics.iter().any(|d| d.code == "E015"));
    assert_eq!(a.file.processes.len(), 2);
    assert_eq!(
        a.file.processes[1].name.as_ref().map(|n| n.name.as_str()),
        Some("B")
    );
}

#[test]
fn text_outside_a_process_is_reported_once_for_the_whole_run() {
    let c = codes("hello world 42\nprocess P { start -> a\n end a }");
    assert_eq!(c, vec!["E012", "E001"]); // the words, and the stray `42` as ONE run, not two errors
    let a = analyze("hello world 42\nprocess P { start -> a\n end a }");
    assert!(
        a.diagnostics[1].message.contains("`42`"),
        "{}",
        a.diagnostics[1].message
    );
}

#[test]
fn missing_open_brace_and_missing_gateway_brace() {
    assert!(has("process P start -> a end a }", "E011"));
    assert!(has(&proc("start -> g\ngateway g\nend e"), "E011"));
}

#[test]
fn empty_and_whitespace_input_are_fine() {
    assert!(analyze("").diagnostics.is_empty());
    assert!(analyze("  \n\t // only a comment\n").diagnostics.is_empty());
}

#[test]
fn oversize_input_is_refused_not_analyzed() {
    let big = "x".repeat(procflow_core::lexer::MAX_SOURCE_BYTES + 1);
    let a = analyze(&big);
    assert_eq!(a.diagnostics.len(), 1);
    assert_eq!(a.diagnostics[0].code, "E900");
    assert!(a.tokens.is_empty());
}

#[test]
fn a_large_valid_program_is_analyzed() {
    let mut s = String::from("process Big {\n start -> t0\n");
    for i in 0..5000 {
        s.push_str(&format!("task t{i} \"step {i}\" -> t{}\n", i + 1));
    }
    s.push_str("end t5000\n}\n");
    assert!(s.len() < procflow_core::lexer::MAX_SOURCE_BYTES);
    let a = analyze(&s);
    assert!(a.diagnostics.is_empty(), "{:?}", a.diagnostics.first());
    assert_eq!(a.graphs[0].depth.last().copied().flatten(), Some(5001));
}

#[test]
fn a_run_of_stray_characters_is_one_diagnostic_and_stops_at_real_tokens() {
    let a = analyze(&proc("start -> a\n@#$ task a -> b\nend b"));
    let stray: Vec<_> = a.diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(stray.len(), 1);
    assert!(stray[0].message.contains("`@#$`"));
    assert_eq!(a.diagnostics.iter().filter(|d| d.code == "E106").count(), 0); // task a is intact
}

#[test]
fn no_follow_on_errors_after_a_syntax_error_in_the_same_process_but_real_ones_remain() {
    // Syntax error on line 3 (stray `@@`), plus a genuinely undefined target and a duplicate name.
    let c = codes(&proc(
        "start -> a\ntask a -> nowhere\n@@\ntask a -> b\nend b",
    ));
    assert!(
        c.contains(&"E001") && c.contains(&"E102") && c.contains(&"E101"),
        "{c:?}"
    );
    // But completeness checks stay quiet until the syntax is fixed.
    let none = codes(&proc("start -> a\n@@\ntask a"));
    assert_eq!(none, vec!["E001"], "{none:?}");
}

#[test]
fn a_clean_process_is_still_checked_for_completeness_when_another_process_has_a_syntax_error() {
    let c = codes("process Bad { start a }\nprocess Good { task x -> y }");
    assert!(c.contains(&"E011"), "{c:?}"); // Bad's syntax error
    assert!(c.contains(&"E103") && c.contains(&"E105"), "{c:?}"); // Good still gets its completeness checks
}
