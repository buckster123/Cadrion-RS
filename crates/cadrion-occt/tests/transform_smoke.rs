//! OCCT translate / rotate / mirror / sphere smoke (H3: no STEP thrash on transforms).
//!
//! Keep as a **single** test — OCCT is happier serial.

use cadrion_kernel::{GeomKernel, Placement, Point3};
use cadrion_lang::{evaluate, execute_ir, EvalOptions};
use cadrion_occt::OcctKernel;

#[test]
fn transform_quality_serial() {
    // translate
    {
        let mut k = OcctKernel::new();
        let s = k
            .box_solid(20.0, 10.0, 5.0, Placement::at(Point3::ORIGIN))
            .unwrap();
        let t = k.translate(s, 30.0, 0.0, 0.0).expect("translate");
        let f = k.facts(t).unwrap();
        assert!(
            (f.volume_mm3 - 1000.0).abs() / 1000.0 < 0.1,
            "translate vol {}",
            f.volume_mm3
        );
        assert!(f.bbox_mm.center().x > 20.0);
    }

    // rotate Z
    {
        let mut k = OcctKernel::new();
        let s = k
            .box_solid(20.0, 10.0, 5.0, Placement::at(Point3::ORIGIN))
            .unwrap();
        let t = k.rotate_about_axis(s, "z", 45.0).expect("rotate");
        let f = k.facts(t).unwrap();
        assert!(
            (f.volume_mm3 - 1000.0).abs() / 1000.0 < 0.1,
            "rotate vol {}",
            f.volume_mm3
        );
    }

    // mirror YZ (x → -x)
    {
        let mut k = OcctKernel::new();
        let s = k
            .box_solid(10.0, 10.0, 10.0, Placement::at(Point3::new(20.0, 0.0, 0.0)))
            .unwrap();
        let m = k.mirror_plane(s, "yz").expect("mirror");
        let f = k.facts(m).unwrap();
        assert!(
            (f.volume_mm3 - 1000.0).abs() / 1000.0 < 0.15,
            "mirror vol {}",
            f.volume_mm3
        );
        assert!(
            f.bbox_mm.center().x < -10.0,
            "mirror center x {}",
            f.bbox_mm.center().x
        );
    }

    // sphere
    {
        let mut k = OcctKernel::new();
        let s = k
            .sphere(10.0, Placement::at(Point3::new(0.0, 0.0, 5.0)))
            .expect("sphere");
        let f = k.facts(s).unwrap();
        let expect = 4.0 / 3.0 * std::f64::consts::PI * 1000.0;
        assert!(
            (f.volume_mm3 - expect).abs() / expect < 0.15,
            "sphere vol {} expect {}",
            f.volume_mm3,
            expect
        );
    }

    // star: translate + rotate_z
    {
        let src = r#"
def gen_step():
    b = box(20.0, 10.0, 5.0, at=CENTER)
    b = translate(b, 30.0, 0.0, 0.0)
    b = rotate_z(b, 45.0)
    return solid(b, label="xf")
"#;
        let r = evaluate(src, &EvalOptions::new("xf.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        let mut k = OcctKernel::new();
        let sid = execute_ir(&mut k, &ir).expect("execute");
        let f = k.facts(sid).expect("facts");
        assert!(
            (f.volume_mm3 - 1000.0).abs() / 1000.0 < 0.1,
            "star vol {}",
            f.volume_mm3
        );
    }

    // H3: transforms must not litter /tmp with cadrion-occt-xf/clone STEP files.
    {
        let before = count_tmp_cadrion_step();
        let mut k = OcctKernel::new();
        let s = k
            .box_solid(5.0, 5.0, 5.0, Placement::at(Point3::ORIGIN))
            .unwrap();
        let s = k.translate(s, 1.0, 2.0, 3.0).unwrap();
        let s = k.rotate_about_axis(s, "z", 90.0).unwrap();
        let _ = k.mirror_plane(s, "xy").unwrap();
        let after = count_tmp_cadrion_step();
        assert_eq!(
            before, after,
            "transform path wrote temp STEP files (H3 regression)"
        );
    }

    // parity part 07 (uses rotate/translate heavily)
    {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../parity/parts/07_finned_cylinder/part.cad.star"
        ))
        .expect("part");
        let r = evaluate(&src, &EvalOptions::new("fin.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
        let ir = r.ir.unwrap();
        let mut k = OcctKernel::new();
        let sid = execute_ir(&mut k, &ir).expect("execute fins");
        let f = k.facts(sid).expect("facts");
        assert!(f.volume_mm3 > 5000.0, "fin volume {}", f.volume_mm3);
        let dir = tempfile::tempdir().unwrap();
        let step = dir.path().join("fin.step");
        k.write_step(sid, &step, &Default::default()).expect("step");
        assert!(step.is_file());
    }
}

fn count_tmp_cadrion_step() -> usize {
    let tmp = std::env::temp_dir();
    let Ok(rd) = std::fs::read_dir(&tmp) else {
        return 0;
    };
    rd.flatten()
        .filter(|e| {
            let n = e.file_name();
            let s = n.to_string_lossy();
            s.starts_with("cadrion-occt-") && s.ends_with(".step")
        })
        .count()
}
