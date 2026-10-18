use hy_check::{check_file, CheckResult};
use hy_syntax::parse_file;

fn check(src: &str) -> CheckResult {
    let f = parse_file("test.hy", src).expect("parse ok");
    check_file(&f)
}

fn error_codes(res: &CheckResult) -> Vec<String> {
    res.diagnostics
        .iter()
        .filter(|d| d.severity == hy_diag::Severity::Error)
        .map(|d| d.code.to_string())
        .collect()
}

fn assert_rejects_e6002(src: &str, label: &str) {
    let r = check(src);
    let codes = error_codes(&r);
    assert_eq!(
        codes,
        vec!["HYB-E6002".to_string()],
        "expected exactly one HYB-E6002 for {label}, got {codes:?} (diags: {:?})",
        r.diagnostics
    );
}

fn assert_accepts(src: &str, label: &str) {
    let r = check(src);
    let codes = error_codes(&r);
    assert!(
        codes.is_empty(),
        "expected 0 diagnostics for {label}, got {codes:?} (diags: {:?})",
        r.diagnostics
    );
}

#[test]
fn issue_725_lead_repro_rejects_both_sites_with_e6002() {
    let src = r#"
package main

const top u8 = 255

fn main() {
    a := top + 1
    b := i64(1) << 63
    fmt.println("{a} {b}")
}
"#;
    let r = check(src);
    let e6002_count = error_codes(&r).iter().filter(|c| *c == "HYB-E6002").count();
    assert_eq!(
        e6002_count, 2,
        "expected 2 HYB-E6002 diagnostics (for `top + 1` and `i64(1) << 63`), got {:?}",
        r.diagnostics
    );
}

#[test]
fn issue_725_typed_const_operator_table_rejects_all() {
    let cases = [
        ("u8(255) + 1", "fn main() {\n    x := u8(255) + 1\n    _ = x\n}\n"),
        (
            "u8(250) + u8(10)",
            "fn main() {\n    x := u8(250) + u8(10)\n    _ = x\n}\n",
        ),
        ("u8(0) - 1", "fn main() {\n    x := u8(0) - 1\n    _ = x\n}\n"),
        ("-u8(1)", "fn main() {\n    x := -u8(1)\n    _ = x\n}\n"),
        ("u8(200) * 2", "fn main() {\n    x := u8(200) * 2\n    _ = x\n}\n"),
        ("u8(1) << 9", "fn main() {\n    x := u8(1) << 9\n    _ = x\n}\n"),
        (
            "u8(1) << u32(8)",
            "fn main() {\n    x := u8(1) << u32(8)\n    _ = x\n}\n",
        ),
        (
            "1 << u32(63)",
            "fn main() {\n    x := 1 << u32(63)\n    _ = x\n}\n",
        ),
        (
            "i8(127) + i8(1)",
            "fn main() {\n    x := i8(127) + i8(1)\n    _ = x\n}\n",
        ),
        (
            "i8(-128) / -1",
            "fn main() {\n    x := i8(-128) / -1\n    _ = x\n}\n",
        ),
        (
            "i64(1) << 63",
            "fn main() {\n    x := i64(1) << 63\n    _ = x\n}\n",
        ),
        (
            "-(i64(1) << 63)",
            "fn main() {\n    x := -(i64(1) << 63)\n    _ = x\n}\n",
        ),
        (
            "int(u8(255) + 1)",
            "fn main() {\n    x := int(u8(255) + 1)\n    _ = x\n}\n",
        ),
        (
            "var b u8 = u8(255) + 1",
            "fn main() {\n    var b u8 = u8(255) + 1\n    _ = b\n}\n",
        ),
        (
            "assignment x = u8(255) + 1",
            "fn main() {\n    var x u8 = 0\n    x = u8(255) + 1\n    _ = x\n}\n",
        ),
        (
            "parenthesized var b u8 = (u8(255) + 1)",
            "fn main() {\n    var b u8 = (u8(255) + 1)\n    _ = b\n}\n",
        ),
        (
            "file-level const b u8 = u8(255) + 1",
            "const b u8 = u8(255) + 1\nfn main() {\n    _ = b\n}\n",
        ),
        (
            "parenthesized file-level const b u8 = (u8(255) + 1)",
            "const b u8 = (u8(255) + 1)\nfn main() {\n    _ = b\n}\n",
        ),
        (
            "local const top u8 = 255; top + 1",
            "fn main() {\n    const top u8 = 255\n    x := top + 1\n    _ = x\n}\n",
        ),
        (
            "untyped const from conversion: const top = u8(255); top + 1",
            "const top = u8(255)\nfn main() {\n    x := top + 1\n    _ = x\n}\n",
        ),
        (
            "chained untyped const does not cascade HYB-E6002",
            "const a = u8(255)\nconst b = a + 1\nfn main() {\n    x := b + 1\n    _ = x\n}\n",
        ),
        (
            "chained local untyped const does not cascade HYB-E6002",
            "fn main() {\n    const a = u8(255)\n    const b = a + 1\n    x := b + 1\n    _ = x\n}\n",
        ),
        (
            "local const referencing file const before local variable shadowing",
            "const top u8 = 255\nfn main() {\n    const c = top\n    top := 1\n    _ = top\n    x := c + 1\n    _ = x\n}\n",
        ),
        (
            "-i8(-128)",
            "fn main() {\n    x := -i8(-128)\n    _ = x\n}\n",
        ),
    ];

    for (label, src) in cases {
        assert_rejects_e6002(src, label);
    }
}

#[test]
fn issue_725_valid_typed_and_untyped_constants_stay_accepted() {
    let cases = [
        (
            "in-range typed u8 addition",
            "fn main() {\n    x := u8(200) + 55\n    _ = x\n}\n",
        ),
        (
            "u64(1) << 63 fits in u64",
            "fn main() {\n    x := u64(1) << 63\n    _ = x\n}\n",
        ),
        (
            "(-i64(1)) << 63 == i64::MIN fits in i64",
            "fn main() {\n    x := (-i64(1)) << 63\n    _ = x\n}\n",
        ),
        (
            "untyped (1 << 63) - 1 fits in int",
            "fn main() {\n    x := (1 << 63) - 1\n    _ = x\n}\n",
        ),
        (
            "untyped -(1 << 63) fits in int",
            "fn main() {\n    x := -(1 << 63)\n    _ = x\n}\n",
        ),
        (
            "SPEC X4 conversion truncation u8(100 + 200)",
            "fn main() {\n    x := u8(100 + 200)\n    _ = x\n}\n",
        ),
        (
            "runtime u8 variable wrapping addition",
            "fn main() {\n    var top u8 = 255\n    x := top + 1\n    _ = x\n}\n",
        ),
        (
            "untyped const top = 255; top + 1",
            "const top = 255\nfn main() {\n    x := top + 1\n    _ = x\n}\n",
        ),
    ];

    for (label, src) in cases {
        assert_accepts(src, label);
    }
}
