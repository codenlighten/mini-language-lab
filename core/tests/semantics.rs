mod common;
use common::*;
use procflow_core::analyze;

#[test]
fn e101_duplicate_names() {
    assert!(has(
        &proc("start -> a\ntask a -> b\ntask a -> b\nend b"),
        "E101"
    ));
    assert!(has(
        &proc("start -> a\ntask a -> a2\nend a\nend a2"),
        "E101"
    )); // an end and a task sharing a name
}

#[test]
fn e102_undefined_target_with_a_close_suggestion() {
    let a = analyze(&proc(
        "start -> a\ntask a -> sihp\ntask ship -> done\nend done",
    ));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "E102")
        .expect("E102");
    assert_eq!(d.hint.as_deref(), Some("did you mean `ship`?"));
}

#[test]
fn e102_gives_no_guess_when_two_names_are_equally_close() {
    let a = analyze(&proc(
        "start -> a\ntask a -> sho\ntask shop -> done\ntask show -> done\nend done",
    ));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "E102")
        .expect("E102");
    assert_eq!(d.hint, None);
}

#[test]
fn e102_gives_no_guess_when_nothing_is_close() {
    let a = analyze(&proc("start -> a\ntask a -> zzzzzz\nend done"));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "E102")
        .expect("E102");
    assert_eq!(d.hint, None);
}

#[test]
fn steps_may_be_used_before_they_are_declared() {
    assert_eq!(
        codes(&proc("start -> later\ntask later -> done\nend done")),
        Vec::<&str>::new()
    );
    assert_eq!(
        codes(&proc("end done\ntask later -> done\nstart -> later")),
        Vec::<&str>::new()
    );
}

#[test]
fn e103_missing_start_and_e104_second_start() {
    assert!(has(&proc("task a -> b\nend b"), "E103"));
    let c = codes(&proc("start -> a\nstart -> b\ntask a -> b\nend b"));
    assert!(c.contains(&"E104"), "{c:?}");
}

#[test]
fn e105_missing_end() {
    assert!(has(&proc("start -> a\ntask a -> a"), "E105"));
}

#[test]
fn e106_task_without_a_next_step() {
    let a = analyze(&proc("start -> a\ntask a\nend b"));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "E106")
        .expect("E106");
    assert!(d.hint.as_deref().unwrap_or("").contains("->"));
}

#[test]
fn e107_gateway_needs_two_branches() {
    assert!(has(
        &proc("start -> g\ngateway g { yes -> e }\nend e"),
        "E107"
    ));
    assert!(has(&proc("start -> g\ngateway g { }\nend e"), "E107"));
}

#[test]
fn w202_duplicate_branch_labels_is_a_warning_not_an_error() {
    let a = analyze(&proc(
        "start -> g\ngateway g { yes -> e, yes -> f }\nend e\nend f",
    ));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "W202")
        .expect("W202");
    assert_eq!(d.severity, procflow_core::diagnostic::Severity::Warning);
}

#[test]
fn w201_unreachable_steps() {
    let a = analyze(&proc("start -> a\ntask a -> e\ntask orphan -> e\nend e"));
    let d = a
        .diagnostics
        .iter()
        .find(|d| d.code == "W201")
        .expect("W201");
    assert!(d.message.contains("orphan"));
    assert!(a.graphs[0].depth[a.graphs[0]
        .nodes
        .iter()
        .position(|n| n.id == "orphan")
        .unwrap()]
    .is_none());
}

#[test]
fn e108_a_loop_with_no_way_out() {
    let c = codes(&proc("start -> a\ntask a -> b\ntask b -> a\nend e"));
    assert!(c.contains(&"E108"), "{c:?}");
    assert!(c.contains(&"W201"), "{c:?}"); // `e` is unreachable too
}

#[test]
fn a_loop_with_a_way_out_is_fine() {
    assert_eq!(
        codes(&proc(
            "start -> a\ntask a -> g\ngateway g { again -> a, stop -> e }\nend e"
        )),
        Vec::<&str>::new()
    );
}

#[test]
fn e110_duplicate_process_names_but_names_are_scoped_per_process() {
    let two = "process A { start -> x\n end x }\nprocess B { start -> x\n end x }";
    assert_eq!(codes(two), Vec::<&str>::new());
    assert!(has(
        "process A { start -> x\n end x }\nprocess A { start -> x\n end x }",
        "E110"
    ));
}

#[test]
fn a_task_may_loop_to_itself_only_if_an_end_is_reachable_otherwise_e108() {
    assert!(has(&proc("start -> a\ntask a -> a\nend e"), "E108"));
}

#[test]
fn depth_is_the_shortest_path_from_start() {
    let a = analyze(SAMPLE);
    let g = &a.graphs[0];
    let d = |id: &str| g.depth[g.nodes.iter().position(|n| n.id == id).unwrap()];
    assert_eq!(
        (
            d("start"),
            d("check"),
            d("decide"),
            d("ship"),
            d("backorder"),
            d("done")
        ),
        (Some(0), Some(1), Some(2), Some(3), Some(3), Some(4))
    );
}

#[test]
fn diagnostics_are_sorted_by_position() {
    let a = analyze(&proc("end e\ntask a -> zz\nstart -> nope"));
    let starts: Vec<usize> = a.diagnostics.iter().map(|d| d.span.start).collect();
    let mut sorted = starts.clone();
    sorted.sort();
    assert_eq!(starts, sorted);
}

#[test]
fn edit_distance_counts_a_swap_of_neighbours_as_one_edit() {
    use procflow_core::validate::edit_distance;
    assert_eq!(edit_distance("ship", "sihp"), 1);
    assert_eq!(edit_distance("ship", "shop"), 1);
    assert_eq!(edit_distance("ship", "shi"), 1);
    assert_eq!(edit_distance("ship", "ships"), 1);
    assert_eq!(edit_distance("", "abc"), 3);
    assert_eq!(edit_distance("abc", "abc"), 0);
    assert_eq!(edit_distance("kitten", "sitting"), 3);
}

#[test]
fn a_typo_in_a_name_does_not_cause_unreachable_or_endless_loop_reports() {
    // `sihp` should be `ship`. Without the rule, `a` would be called endless and `ship`/`done` unreachable.
    let c = codes(&proc(
        "start -> a\ntask a -> sihp\ntask ship -> done\nend done",
    ));
    assert_eq!(c, vec!["E102"], "{c:?}");
}

#[test]
fn whole_graph_checks_return_once_every_name_resolves() {
    let c = codes(&proc(
        "start -> a\ntask a -> ship\ntask ship -> done\ntask orphan -> done\nend done",
    ));
    assert_eq!(c, vec!["W201"], "{c:?}");
}

#[test]
fn declaration_checks_still_run_when_a_name_does_not_resolve() {
    // A missing end and a duplicate name are facts about the declarations, so a typo does not hide them.
    let c = codes(&proc("start -> a\ntask a -> nowhere\ntask a -> nowhere"));
    assert!(
        c.contains(&"E102") && c.contains(&"E101") && c.contains(&"E105"),
        "{c:?}"
    );
}

#[test]
fn a_task_without_a_next_step_does_not_cause_unreachable_or_endless_loop_reports() {
    // The missing successor is the only real problem: `a` is not "endless" and `done` is not "unreachable".
    let c = codes(&proc("start -> a\ntask a\nend done"));
    assert_eq!(c, vec!["E106"], "{c:?}");
}

#[test]
fn a_gateway_with_no_branches_does_not_cause_follow_on_graph_reports() {
    // With no branches the edges to `a` and `done` are missing, so they would look unreachable.
    let c = codes(&proc("start -> g\ngateway g { }\ntask a -> done\nend done"));
    assert_eq!(c, vec!["E107"], "{c:?}");
}

#[test]
fn a_gateway_with_one_branch_is_reported_and_graph_checks_still_run() {
    // Its single edge exists, so nothing is missing: the orphan is still found.
    let c = codes(&proc(
        "start -> g\ngateway g { only -> done }\ntask orphan -> done\nend done",
    ));
    assert!(c.contains(&"E107") && c.contains(&"W201"), "{c:?}");
}

#[test]
fn a_very_long_name_cannot_exhaust_memory_in_the_did_you_mean_search() {
    let long = "a".repeat(30_000);
    let other = "b".repeat(30_000);
    let src = format!("process P {{ start -> {long} end {other} }}");
    let t = std::time::Instant::now();
    let a = analyze(&src);
    assert!(a
        .diagnostics
        .iter()
        .any(|d| d.code == "E102" && d.hint.is_none()));
    assert!(t.elapsed().as_secs() < 5, "took {:?}", t.elapsed());
}

#[test]
fn suggestions_still_work_for_ordinary_names_after_the_length_cap() {
    use procflow_core::validate::suggest;
    assert_eq!(
        suggest("recieve", ["receive", "send"].into_iter()),
        Some("receive")
    );
    assert_eq!(suggest(&"x".repeat(100), ["x"].into_iter()), None);
    assert_eq!(suggest("abc", ["abcdefghij"].into_iter()), None); // too different in length to be a typo
                                                                  // A 100-letter name one letter off a known 100-letter name is NOT compared: suggestions are capped.
    let (wanted, known) = ("x".repeat(99) + "y", "x".repeat(100));
    assert_eq!(suggest(&wanted, [known.as_str()].into_iter()), None);
}

#[test]
fn edit_distance_agrees_with_a_slow_reference_on_many_pairs() {
    use procflow_core::validate::edit_distance;
    // Reference: plain full-table optimal string alignment.
    fn reference(a: &str, b: &str) -> usize {
        let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
        let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
        for (i, row) in d.iter_mut().enumerate() {
            row[0] = i;
        }
        for (j, cell) in d[0].iter_mut().enumerate() {
            *cell = j;
        }
        for i in 1..=a.len() {
            for j in 1..=b.len() {
                let c = usize::from(a[i - 1] != b[j - 1]);
                d[i][j] = (d[i - 1][j - 1] + c)
                    .min(d[i - 1][j] + 1)
                    .min(d[i][j - 1] + 1);
                if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                    d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
                }
            }
        }
        d[a.len()][b.len()]
    }
    let mut r = 0x1234_5678_9abc_def0u64;
    let mut next = move || {
        r ^= r << 13;
        r ^= r >> 7;
        r ^= r << 17;
        r
    };
    for _ in 0..3000 {
        let mk = |n: &mut dyn FnMut() -> u64| -> String {
            (0..(n() % 9))
                .map(|_| (b'a' + (n() % 4) as u8) as char)
                .collect()
        };
        let (a, b) = (mk(&mut next), mk(&mut next));
        assert_eq!(edit_distance(&a, &b), reference(&a, &b), "{a:?} vs {b:?}");
    }
}
