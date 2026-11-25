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
