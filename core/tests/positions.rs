mod common;
use common::*;
use procflow_core::source::{Position, Source, Span};
use procflow_core::{analysis_json, analyze};

/// The UTF-16 slice of `text` between two UTF-16 offsets.
fn slice16(text: &str, start: usize, end: usize) -> String {
    let u: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&u[start..end]).expect("offsets must fall on character boundaries")
}

#[test]
fn utf16_offsets_differ_from_byte_offsets_after_multibyte_characters() {
    let text = "// héllo 😀\nprocess P { start -> a\n end a }";
    let s = Source::new(text);
    let byte = text.find("process").unwrap();
    let p = s.position(byte);
    assert_eq!(
        p,
        Position {
            line: 1,
            column: 0,
            offset: text[..byte].encode_utf16().count()
        }
    );
    assert!(p.offset < byte, "bytes {byte} vs utf16 {}", p.offset); // é is 2 bytes, 😀 is 4 bytes but 2 units
}

#[test]
fn a_surrogate_pair_counts_as_two_columns() {
    let text = "😀x";
    let s = Source::new(text);
    assert_eq!(s.position(4).column, 2); // after the emoji
    assert_eq!(s.position(5).column, 3);
}

#[test]
fn positions_at_the_end_and_past_it_are_clamped() {
    let s = Source::new("ab\ncd");
    assert_eq!(
        s.position(5),
        Position {
            line: 1,
            column: 2,
            offset: 5
        }
    );
    assert_eq!(
        s.position(999),
        Position {
            line: 1,
            column: 2,
            offset: 5
        }
    );
    assert_eq!(
        s.position(0),
        Position {
            line: 0,
            column: 0,
            offset: 0
        }
    );
    assert_eq!(
        s.position(3),
        Position {
            line: 1,
            column: 0,
            offset: 3
        }
    );
}

#[test]
fn slice_returns_the_text_of_a_span() {
    let s = Source::new("hello world");
    assert_eq!(s.slice(Span::new(6, 11)), "world");
}

#[test]
fn json_offsets_select_the_same_text_as_the_byte_spans() {
    let text =
        "// 😀 comment\nprocess P {\n  start -> á\n  task é -> done\n  end done \"✓ ok\"\n}\n";
    let a = analyze(text);
    let json = analysis_json(text, &a);
    assert!(is_valid_json(&json), "{json}");
    // Every token's UTF-16 range must slice the same string as its byte range.
    let src = Source::new(text);
    for t in &a.tokens {
        let (s, e) = (
            src.position(t.span.start).offset,
            src.position(t.span.end).offset,
        );
        assert_eq!(slice16(text, s, e), &text[t.span.start..t.span.end]);
    }
    assert!(a.diagnostics.iter().any(|d| d.code == "E001"));
}

#[test]
fn json_is_valid_for_awkward_text_in_messages_and_labels() {
    let text = "process P {\n start -> a\n task a \"quote \\\" slash \\\\ nl \\n tab\t ctl\u{1}\" -> \"bad\n end e }\n";
    let json = analysis_json(text, &analyze(text));
    assert!(is_valid_json(&json), "{json}");
}

#[test]
fn json_has_the_documented_shape() {
    let json = analysis_json(SAMPLE, &analyze(SAMPLE));
    for key in [
        "\"tokens\"",
        "\"diagnostics\"",
        "\"processes\"",
        "\"nodes\"",
        "\"edges\"",
        "\"depth\"",
    ] {
        assert!(json.contains(key), "missing {key}");
    }
    assert!(
        json.contains("\"label\":\"yes\"") && json.contains("\"from\":\"decide\",\"to\":\"ship\"")
    );
}

#[test]
fn diagnostic_line_and_column_are_zero_based_and_correct() {
    let text = "process P {\n  start -> a\n  task a -> nope\n  end e\n}";
    let a = analyze(text);
    let d = a.diagnostics.iter().find(|d| d.code == "E102").unwrap();
    let json = analysis_json(text, &a);
    assert!(
        json.contains("\"line\":2,\"column\":12,\"endLine\":2,\"endColumn\":16"),
        "{json}"
    );
    assert_eq!(&text[d.span.start..d.span.end], "nope");
}
