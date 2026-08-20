//! H5-7 / OQ-2 dialect goldens: float formatting + stdlib names.

use cadrion_lang::{evaluate, EvalOptions, STDLIB_SYMBOLS};

const BOX_INT_FLOAT: &str = include_str!("../fixtures/dialect/01_int_and_float.cad.star");
const BOX_INT_FLOAT_IR: &str = include_str!("../fixtures/dialect/01_int_and_float.ir.json");

#[test]
fn int_and_float_literals_share_ir_golden() {
    let a = evaluate(
        BOX_INT_FLOAT,
        &EvalOptions::new("01_int_and_float.cad.star"),
    );
    assert!(a.ok, "{:?}", a.diagnostics);
    let ir = a.ir.unwrap();
    assert_eq!(ir.golden_json(), BOX_INT_FLOAT_IR);

    let b = evaluate(
        r#"
def gen_step():
    return solid(box(10.0, 20.0, 30.0, at=CENTER), label="cube")
"#,
        &EvalOptions::new("floats.cad.star"),
    );
    assert!(b.ok, "{:?}", b.diagnostics);
    assert_eq!(b.ir.unwrap(), ir);
}

#[test]
fn stdlib_symbols_are_sorted_unique_and_live() {
    let mut sorted = STDLIB_SYMBOLS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, STDLIB_SYMBOLS);
    assert!(STDLIB_SYMBOLS.contains(&"box"));
    assert!(STDLIB_SYMBOLS.contains(&"cylinder"));
    assert!(STDLIB_SYMBOLS.contains(&"solid"));
    assert!(!STDLIB_SYMBOLS.contains(&"CENTER"));
    assert!(!STDLIB_SYMBOLS.contains(&"load"));
    assert!(!STDLIB_SYMBOLS.contains(&"use"));

    let src = r#"
def gen_step():
    a = box(10.0, 10.0, 10.0, at=CENTER)
    b = cylinder(2.0, 10.0, at=CENTER)
    c = sphere(3.0, at=CENTER)
    d = cone(4.0, 8.0, at=CENTER)
    e = translate(a, 1.0, 0.0, 0.0)
    f = rotate_z(e, 90.0)
    g = mirror(f, "xy")
    h = union(g, b)
    i = cut(h, c)
    j = intersect(i, d)
    return solid(j, label="dialect")
"#;
    let r = evaluate(src, &EvalOptions::new("names.cad.star"));
    assert!(r.ok, "{:?}", r.diagnostics);
    assert_eq!(r.ir.unwrap().label.as_deref(), Some("dialect"));
}

#[test]
fn unknown_symbol_and_use_are_not_stdlib() {
    let unknown = evaluate(
        "def gen_step():\n    return not_a_cadrion_prim(1.0)\n",
        &EvalOptions::new("unknown.cad.star"),
    );
    assert!(!unknown.ok);
    let use_mod = evaluate(
        "use(\"cadrion.patterns\")\ndef gen_step():\n    return solid(box(1.0, 1.0, 1.0), label=\"x\")\n",
        &EvalOptions::new("use.cad.star"),
    );
    assert!(
        !use_mod.ok,
        "use() is not shipped; names are already global"
    );
}

#[test]
fn load_still_refused() {
    let r = evaluate(
        "load(\"lib.star\", \"x\")\ndef gen_step():\n    return box(1,1,1)\n",
        &EvalOptions::new("load.cad.star"),
    );
    assert!(!r.ok);
    assert_eq!(r.diagnostics[0].code, "CADRION-E-HERMETIC-LOAD");
}
