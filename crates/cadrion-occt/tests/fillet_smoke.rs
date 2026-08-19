//! Fillet / chamfer diagnostics + happy path (OCCT).

use cadrion_kernel::{EdgeRef, GeomKernel, Placement, Point3};
use cadrion_lang::{evaluate, execute_ir, EvalOptions};
use cadrion_occt::OcctKernel;

#[test]
fn fillet_all_edges_reduces_sharp_box() {
    let mut k = OcctKernel::new();
    let s = k
        .box_solid(20.0, 20.0, 20.0, Placement::at(Point3::ORIGIN))
        .unwrap();
    let f0 = k.facts(s).unwrap();
    let filleted = k.fillet(s, &[], 2.0).expect("fillet");
    let f1 = k.facts(filleted).unwrap();
    assert!(
        f1.volume_mm3 < f0.volume_mm3,
        "fillet should remove volume: before {} after {}",
        f0.volume_mm3,
        f1.volume_mm3
    );
    assert!(f1.faces > f0.faces, "fillet adds faces");
}

#[test]
fn chamfer_all_edges_reduces_volume() {
    let mut k = OcctKernel::new();
    let s = k
        .box_solid(20.0, 20.0, 20.0, Placement::at(Point3::ORIGIN))
        .unwrap();
    let f0 = k.facts(s).unwrap();
    let c = k.chamfer(s, &[], 2.0).expect("chamfer");
    let f1 = k.facts(c).unwrap();
    assert!(f1.volume_mm3 < f0.volume_mm3);
}

#[test]
fn impossible_fillet_radius_is_structured_diagnostic() {
    let mut k = OcctKernel::new();
    // Tiny box, huge radius → OCCT fails IsDone
    let s = k
        .box_solid(4.0, 4.0, 4.0, Placement::at(Point3::ORIGIN))
        .unwrap();
    let err = k.fillet(s, &[], 50.0).unwrap_err();
    assert_eq!(err.code(), "CADRION-E-FILLET-FAILED");
    let json = serde_json::to_value(&err).unwrap();
    let refs = json.get("refs").and_then(|r| r.as_array()).unwrap();
    assert!(!refs.is_empty(), "should name edge selectors");
    assert!(
        refs.iter().any(|r| r.as_str() == Some("#e0")),
        "expected #e0 in {refs:?}"
    );
}

#[test]
fn unknown_edge_index_diagnostic() {
    let mut k = OcctKernel::new();
    let s = k
        .box_solid(10.0, 10.0, 10.0, Placement::at(Point3::ORIGIN))
        .unwrap();
    let err = k.fillet(s, &[EdgeRef(999)], 1.0).unwrap_err();
    assert_eq!(err.code(), "CADRION-E-UNKNOWN-EDGE");
}

#[test]
fn cone_is_unsupported_no_cylinder_standin() {
    let mut k = OcctKernel::new();
    let err = k
        .cone(10.0, 20.0, Placement::at(Point3::ORIGIN))
        .unwrap_err();
    assert_eq!(err.code(), "CADRION-E-UNSUPPORTED");
    let msg = err.to_string();
    assert!(msg.contains("cone"), "diagnostic should name cone: {msg}");
}

#[test]
fn star_fillet_and_chamfer_roundtrip() {
    let src = r#"
P = params(fillet_r=1.5, cham=1.0)
def gen_step():
    b = box(30.0, 20.0, 10.0, at=CENTER)
    b = fillet(b, radius=P.fillet_r)
    # second solid for chamfer demo
    c = box(15.0, 15.0, 15.0, at=(40.0, 0.0, 0.0))
    c = chamfer(c, distance=P.cham)
    return solid(union(b, c), label="fc")
"#;
    let r = evaluate(src, &EvalOptions::new("fc.cad.star"));
    assert!(r.ok, "{:?}", r.diagnostics);
    let ir = r.ir.unwrap();
    let mut k = OcctKernel::new();
    let sid = execute_ir(&mut k, &ir).expect("execute");
    let f = k.facts(sid).unwrap();
    assert!(f.volume_mm3 > 1000.0);
    assert!(f.faces > 12);
}
