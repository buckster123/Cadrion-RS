//! Cadrion-RS facade crate.
//!
//! Re-exports workspace libraries. Logic lives in `cadrion-*` crates
//! (`docs/design.md`). Binding decisions: `docs/CHARTER.md`.

#![deny(unsafe_code)]

pub use cadrion_inspect as inspect;
pub use cadrion_kernel as kernel;
pub use cadrion_lang as lang;
pub use cadrion_model as model;

/// Workspace facade version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use cadrion_inspect::{box_topology, inspect_refs, TopologySnapshot};
    use cadrion_kernel::{GeomKernel, MockKernel, Placement, Point3};
    use cadrion_lang::{evaluate, EvalOptions};
    use cadrion_model::{parse_selector, BuildCache, CacheKey};

    #[test]
    fn version_is_semverish() {
        let v = super::VERSION;
        assert!(v.split('.').count() >= 2, "version={v}");
    }

    #[test]
    fn facade_reexports_kernel() {
        let mut k = MockKernel::new();
        let id = k
            .box_solid(1.0, 2.0, 3.0, Placement::IDENTITY)
            .expect("box");
        let f = k.facts(id).expect("facts");
        assert!((f.volume_mm3 - 6.0).abs() < 1e-12);
    }

    #[test]
    fn facade_reexports_lang() {
        let src = r#"
def gen_step():
    return solid(box(1.0, 2.0, 3.0, at=CENTER), label="x")
"#;
        let r = evaluate(src, &EvalOptions::new("facade.cad.star"));
        assert!(r.ok, "{:?}", r.diagnostics);
    }

    #[test]
    fn facade_reexports_model_inspect() {
        assert_eq!(parse_selector("#o1.1.f2").unwrap().to_string(), "#o1.1.f2");
        let snap = TopologySnapshot::single_solid(box_topology(10.0, 10.0, 10.0, Point3::ORIGIN));
        let r = inspect_refs(&snap, true);
        assert_eq!(r.faces, 6);
        let dir = std::env::temp_dir().join(format!("cadrion-facade-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cache = BuildCache::open(&dir).unwrap();
        let key = CacheKey::from_source("s", "{}", "0.1.0", "mock", "0", None);
        cache.put(&key, b"x", "a.step", None, None).unwrap();
        assert!(cache.get(&key).unwrap().is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
