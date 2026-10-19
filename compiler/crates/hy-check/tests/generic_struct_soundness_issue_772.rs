use hy_check::check_file;
use hy_syntax::parse_file;

fn codes(src: &str) -> Vec<String> {
    let file = parse_file("test.hy", src).expect("unexpected parse errors");
    check_file(&file)
        .diagnostics
        .into_iter()
        .map(|d| d.code.to_string())
        .collect()
}

#[test]
fn generic_struct_tier0_alias_freeze_and_non_owned_move_rejected() {
    let src = r#"package main

struct box[t] {
    v t
}

fn worker(b box[[]int], done chan<- bool) {
    b.v[0] = 1
    done <- true
}

fn leak_move(b box[[]int]) []int {
    return move b.v
}

fn main() {
    done := make(chan bool, 1)
    a := box[[]int]{v: []int{10, 20}}
    b := a
    task worker(move a, done)
    b.v[0] = 99
    <-done
    _ = leak_move(b)
}
"#;
    let got = codes(src);
    assert!(
        got.iter().any(|c| c == "HYB-E3003"),
        "expected HYB-E3003 for mutating alias `b` of generic struct holding slice after `move a` into task, got {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "HYB-E3004"),
        "expected HYB-E3004 for moving slice out of non-owned generic struct parameter, got {got:?}"
    );
}

#[test]
fn generic_struct_const_deep_mutation_rejected() {
    let src = r#"package main

struct box[t] {
    v t
}

const cb box[[]int] = box[[]int]{v: []int{1, 2, 3}}

fn main() {
    local := cb
    local.v[0] = 777
    alias := cb.v
    alias[1] = 888
}
"#;
    let got = codes(src);
    let e3006_count = got.iter().filter(|c| c.as_str() == "HYB-E3006").count();
    assert!(
        e3006_count >= 2,
        "expected HYB-E3006 on both local.v[0] and alias[1] writes into const box[[]int], got {got:?}"
    );
}

#[test]
fn generic_struct_move_required_and_use_after_move_rejected() {
    let src = r#"package main

struct box[t] {
    v t
}

fn worker(b box[[]int], done chan<- bool) {
    done <- true
}

fn main() {
    done := make(chan bool, 2)
    ch := make(chan box[[]int], 1)
    a := box[[]int]{v: []int{1, 2}}
    task worker(a, done)
    ch <- a

    b := box[[]int]{v: []int{3, 4}}
    task worker(move b, done)
    _ = b.v[0]
    <-done
}
"#;
    let got = codes(src);
    assert!(
        got.iter().any(|c| c == "HYB-E3001"),
        "expected HYB-E3001 for spawning task with box[[]int] without `move`, got {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "HYB-E3002"),
        "expected HYB-E3002 for sending box[[]int] over channel without `move`, got {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "HYB-E3003"),
        "expected HYB-E3003 for reusing generic struct box[[]int] after move, got {got:?}"
    );
}

#[test]
fn generic_struct_map_slot_loan_a3_rejected() {
    let src = r#"package main

struct svc {
    count int
}

struct box[t] {
    v t
}

fn main() {
    b := box[map[string]svc]{v: map[string]svc{"a": svc{count: 1}}}
    s := &b.v["a"]
    b.v["b"] = svc{count: 2}
    s.count = 99
}
"#;
    let got = codes(src);
    assert!(
        got.iter().any(|c| c == "HYB-E3102"),
        "expected HYB-E3102 when inserting into map field of generic struct while slot loan `&b.v[\"a\"]` is live, got {got:?}"
    );
}

#[test]
fn generic_struct_n1_nil_field_and_n3_no_zero_value_rejected() {
    let src = r#"package main

enum msg {
    data(int)
    empty
}

struct box[t] {
    v t
}

fn main() {
    b := box[[]int]{v: nil}
    b.v = nil
    var u box[msg]
    _ = b
    _ = u
}
"#;
    let got = codes(src);
    let e4001_count = got.iter().filter(|c| c.as_str() == "HYB-E4001").count();
    assert!(
        e4001_count >= 2,
        "expected HYB-E4001 on both struct literal nil field and field nil assignment on box[[]int], got {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "HYB-E4002"),
        "expected HYB-E4002 on uninitialized `var u box[msg]` where `msg` has no zero value, got {got:?}"
    );
}

#[test]
fn generic_struct_pure_value_instantiation_allows_copy_and_zero_value() {
    let src = r#"package main

struct box[t] {
    v t
}

const cb box[int] = box[int]{v: 42}

fn main() {
    var z box[int]
    a := box[int]{v: 10}
    b := a
    a.v = 20
    local := cb
    local.v = 99
    _ = z.v + b.v + local.v
}
"#;
    let got = codes(src);
    assert!(
        got.is_empty(),
        "expected pure-value generic struct `box[int]` to allow copy, mutation of copy, and zero-value var, got {got:?}"
    );
}
