# cadrion-kernel

`GeomKernel` trait + shared geometry types for Cadrion-RS.

Pure Rust, **no OCCT link**. Backends (`cadrion-occt`, later `cadrion-truck`) implement
the trait. See `docs/occt-binding.md` and `docs/design.md`.

```rust
use cadrion_kernel::{GeomKernel, MockKernel, Point3};

let mut k = MockKernel::new();
let s = k.box_solid(10.0, 20.0, 30.0, Point3::ORIGIN).unwrap();
let f = k.facts(s).unwrap();
assert!((f.volume_mm3 - 6000.0).abs() < 1e-6);
```
