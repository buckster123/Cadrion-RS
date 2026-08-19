use cadrion_kernel::{BooleanOp, GeomKernel, Point3};
use cadrion_occt::OcctKernel;

#[test]
fn union_two_boxes() {
    let mut k = OcctKernel::new();
    let a = k.box_at(10.0, 10.0, 10.0, Point3::ORIGIN).unwrap();
    let b = k
        .box_at(10.0, 10.0, 10.0, Point3::new(5.0, 0.0, 0.0))
        .unwrap();
    let u = k.boolean(BooleanOp::Union, a, b).expect("union");
    let f = k.facts(u).unwrap();
    assert!(f.volume_mm3 > 1000.0, "vol={}", f.volume_mm3);
    let snap = k.topology_snapshot(u).unwrap();
    assert!(snap.solids[0].faces.len() >= 6);
}
