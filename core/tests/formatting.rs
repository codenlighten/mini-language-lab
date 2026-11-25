mod common;
use common::*;
use procflow_core::ast::{File, Item};
use procflow_core::{analyze, format};

/// The program's structure without positions, to compare before and after formatting.
fn shape(f: &File) -> String {
    let mut out = String::new();
    for p in &f.processes {
        out.push_str(&format!("process {:?}\n", p.name.as_ref().map(|n| &n.name)));
        for it in &p.items {
            match it {
                Item::Start { target, .. } => {
                    out.push_str(&format!(" start {:?}\n", target.as_ref().map(|t| &t.name)))
                }
                Item::End { id, label, .. } => out.push_str(&format!(
                    " end {} {:?}\n",
                    id.name,
                    label.as_ref().map(|l| &l.value)
                )),
                Item::Task {
                    id, label, next, ..
                } => out.push_str(&format!(
                    " task {} {:?} {:?}\n",
                    id.name,
                    label.as_ref().map(|l| &l.value),
                    next.as_ref().map(|n| &n.name)
                )),
                Item::Gateway {
                    id,
                    label,
                    branches,
                    ..
                } => {
                    out.push_str(&format!(
                        " gateway {} {:?}\n",
                        id.name,
                        label.as_ref().map(|l| &l.value)
                    ));
                    for b in branches {
                        out.push_str(&format!(
                            "  {} -> {:?}\n",
                            b.label.name,
                            b.target.as_ref().map(|t| &t.name)
                        ));
                    }
                }
            }
        }
    }
    out
}

fn comment_count(s: &str) -> usize {
    analyze(s)
        .tokens
        .iter()
        .filter(|t| t.kind == procflow_core::lexer::TokenKind::Comment)
        .count()
}

#[test]
fn the_sample_is_already_canonical() {
    assert_eq!(format(SAMPLE).as_deref(), Some(SAMPLE));
}

#[test]
fn messy_input_is_normalised() {
    let messy = "process   Order{start->check\n\n\n   task check   \"Check\"  ->  decide\ngateway decide \"Q\" {yes->ship,no->back}\n task ship->done task back->check end done}";
    let want = "process Order {\n  start -> check\n  task check \"Check\" -> decide\n  gateway decide \"Q\" {\n    yes -> ship,\n    no -> back\n  }\n  task ship -> done\n  task back -> check\n  end done\n}\n";
    assert_eq!(format(messy).as_deref(), Some(want));
}

#[test]
fn formatting_is_idempotent_and_keeps_the_structure() {
    let messy = "process A{start->x task x->y end y}process B{start->p end p}";
    let once = format(messy).expect("formats");
    let twice = format(&once).expect("formats again");
    assert_eq!(once, twice);
    assert_eq!(shape(&analyze(messy).file), shape(&analyze(&once).file));
}

#[test]
fn text_with_syntax_errors_is_not_formatted() {
    assert_eq!(format("process P { start a }"), None);
    assert_eq!(format("process P { task a \"open }"), None);
}

#[test]
fn meaning_errors_do_not_block_formatting() {
    let src = "process P{start->missing end e}";
    assert!(format(src).is_some());
}

#[test]
fn labels_with_quotes_backslashes_and_newlines_survive() {
    let src = "process P { start -> a\n task a \"say \\\"hi\\\" \\\\ then\\nstop\" -> e\n end e }";
    let out = format(src).unwrap();
    assert_eq!(shape(&analyze(src).file), shape(&analyze(&out).file));
    assert!(out.contains(r#"\"hi\""#) && out.contains(r"\\") && out.contains(r"\n"));
}

#[test]
fn comments_on_their_own_lines_stay_above_the_next_item() {
    let src = "// top\nprocess P {\n // why we start here\n start -> a\n task a -> e\n end e\n}\n";
    let out = format(src).unwrap();
    assert!(
        out.starts_with("// top\nprocess P {\n  // why we start here\n  start -> a\n"),
        "{out}"
    );
}

#[test]
fn trailing_comments_stay_on_their_line() {
    let src = "process P { start -> a // go\n task a -> e // work\n end e // done\n}";
    let out = format(src).unwrap();
    assert!(
        out.contains("start -> a  // go\n")
            && out.contains("task a -> e  // work\n")
            && out.contains("end e  // done\n"),
        "{out}"
    );
}

#[test]
fn comments_before_a_closing_brace_stay_inside_the_process() {
    let out = format("process P { start -> a\n end a\n // last words\n}").unwrap();
    assert!(out.contains("  end a\n  // last words\n}"), "{out}");
}

#[test]
fn comments_inside_gateway_braces_are_kept_just_below_the_gateway() {
    let src = "process P { start -> g\n gateway g {\n // the happy path\n yes -> e,\n no -> e\n }\n end e }";
    let out = format(src).unwrap();
    assert_eq!(comment_count(&out), 1);
    assert!(out.contains("// the happy path"), "{out}");
}

#[test]
fn comments_after_the_last_process_are_kept() {
    let out = format("process P { start -> a\n end a }\n// the end\n").unwrap();
    assert!(out.ends_with("// the end\n"), "{out}");
}

#[test]
fn comment_count_is_preserved_in_every_position() {
    let src = "// 1\nprocess /*not a comment*/ P { // 2\n // 3\n start -> a // 4\n // 5\n task a -> e\n gateway g { // 6\n yes -> e, // 7\n no -> e } // 8\n end e\n // 9\n} // 10\n// 11\n";
    // `/* */` is not a Procflow comment: it is stray characters, so this text has syntax errors.
    assert_eq!(format(src), None);
    let ok = "// 1\nprocess P { // 2\n // 3\n start -> a // 4\n // 5\n task a -> g\n gateway g { // 6\n yes -> e, // 7\n no -> e } // 8\n end e\n // 9\n} // 10\n// 11\n";
    let out = format(ok).unwrap();
    assert_eq!(comment_count(&out), comment_count(ok));
    assert_eq!(comment_count(ok), 11);
}

#[test]
fn blank_line_between_processes() {
    let out = format("process A{start->x end x}process B{start->y end y}").unwrap();
    assert!(out.contains("}\n\nprocess B {"), "{out}");
}

#[test]
fn a_comment_stays_with_the_construct_it_follows_not_with_an_earlier_item_on_the_same_line() {
    let src =
        "process P { start -> g gateway g { // branch note\n yes -> a, no -> b } end a end b }";
    let out = format(src).unwrap();
    // The note must not end up trailing `start -> g`.
    assert!(!out.contains("start -> g  //"), "{out}");
    assert!(out.contains("// branch note"), "{out}");
    assert_eq!(comment_count(&out), 1);
}

#[test]
fn a_comment_on_the_same_line_right_after_an_item_is_still_a_trailing_comment() {
    let out = format("process P { start -> a   // go\n end a }").unwrap();
    assert!(out.contains("start -> a  // go\n"), "{out}");
}
