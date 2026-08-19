//! CAD stdlib exposed to Starlark (params, box, cylinder, booleans, solid).
//!
//! Side channel is a thread-local [`EvalStore`] so we stay `forbid(unsafe_code)`.

use std::cell::RefCell;
use std::collections::BTreeMap;

use starlark::collections::SmallMap;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::values::float::StarlarkFloat;
use starlark::values::list::ListRef;
use starlark::values::none::NoneType;
use starlark::values::tuple::UnpackTuple;
use starlark::values::{UnpackValue, Value};

use crate::ir::{BooleanKind, IrBuilder, IrNode, NodeId};

/// Evaluation side-channel: IR builder + param overrides.
#[derive(Debug)]
pub struct EvalStore {
    pub builder: IrBuilder,
    pub overrides: BTreeMap<String, f64>,
    params_called: bool,
}

impl EvalStore {
    pub fn new(overrides: BTreeMap<String, f64>) -> Self {
        Self {
            builder: IrBuilder::default(),
            overrides,
            params_called: false,
        }
    }
}

thread_local! {
    static STORE: RefCell<Option<EvalStore>> = const { RefCell::new(None) };
}

/// Install store for the duration of `f` (single-threaded eval).
pub fn with_store<R>(store: EvalStore, f: impl FnOnce() -> R) -> (R, EvalStore) {
    STORE.with(|slot| {
        *slot.borrow_mut() = Some(store);
    });
    let result = f();
    let store = STORE.with(|slot| {
        slot.borrow_mut()
            .take()
            .expect("EvalStore missing after eval")
    });
    (result, store)
}

fn with_builder_mut<R>(f: impl FnOnce(&mut EvalStore) -> anyhow::Result<R>) -> anyhow::Result<R> {
    STORE.with(|slot| {
        let mut borrow = slot.borrow_mut();
        let store = borrow
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("internal: EvalStore not installed"))?;
        f(store)
    })
}

fn value_f64(v: Value<'_>, name: &str) -> anyhow::Result<f64> {
    if let Some(i) = v.unpack_i32() {
        return Ok(i as f64);
    }
    if let Ok(Some(f)) = StarlarkFloat::unpack_value(v) {
        return Ok(f.0);
    }
    if let Ok(s) = v.to_json() {
        if let Ok(n) = s.parse::<f64>() {
            return Ok(n);
        }
    }
    anyhow::bail!("{name}: expected number, got {}", v.get_type())
}

fn parse_at(at: Option<Value<'_>>) -> anyhow::Result<[f64; 3]> {
    let Some(v) = at else {
        return Ok([0.0, 0.0, 0.0]);
    };
    if let Ok(Some(t)) = UnpackTuple::<Value>::unpack_value(v) {
        let items = t.items;
        if items.len() != 3 {
            anyhow::bail!("at= expects a 3-tuple (x,y,z), got len {}", items.len());
        }
        return Ok([
            value_f64(items[0], "at.x")?,
            value_f64(items[1], "at.y")?,
            value_f64(items[2], "at.z")?,
        ]);
    }
    anyhow::bail!("at= expects a 3-tuple (x,y,z), got {}", v.get_type())
}

fn require_positive(name: &str, v: f64) -> anyhow::Result<()> {
    if !v.is_finite() || v <= 0.0 {
        anyhow::bail!("{name} must be finite and > 0, got {v}");
    }
    Ok(())
}

fn shape_id(v: Value<'_>) -> anyhow::Result<NodeId> {
    let i = v
        .unpack_i32()
        .ok_or_else(|| anyhow::anyhow!("shape id must be int, got {}", v.get_type()))?;
    if i < 0 {
        anyhow::bail!("shape id must be >= 0, got {i}");
    }
    Ok(NodeId(i as u32))
}

/// Register CAD primitives on a [`GlobalsBuilder`].
pub fn register_stdlib(builder: &mut GlobalsBuilder) {
    cadrion_stdlib(builder);
}

#[starlark_module]
fn cadrion_stdlib(builder: &mut GlobalsBuilder) {
    /// Declare named numeric parameters. Call at most once. Host overrides win.
    fn params<'v>(
        #[starlark(kwargs)] kwargs: SmallMap<String, Value<'v>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> anyhow::Result<Value<'v>> {
        let heap = eval.heap();
        with_builder_mut(|store| {
            if store.params_called {
                anyhow::bail!("params() may only be called once per module");
            }
            store.params_called = true;

            let mut map = BTreeMap::new();
            for (k, v) in kwargs.iter() {
                map.insert(k.clone(), value_f64(*v, k)?);
            }
            for (k, v) in &store.overrides {
                map.insert(k.clone(), *v);
            }
            store.builder.params = map.clone();

            let mut pairs = Vec::with_capacity(map.len());
            for (k, v) in &map {
                pairs.push((k.as_str(), heap.alloc(*v)));
            }
            Ok(heap.alloc(starlark::values::structs::AllocStruct(pairs)))
        })
    }

    /// Axis-aligned box centered at `at` (default CENTER).
    fn r#box<'v>(
        dx: Value<'v>,
        dy: Value<'v>,
        dz: Value<'v>,
        at: Option<Value<'v>>,
    ) -> anyhow::Result<i32> {
        let dx = value_f64(dx, "dx")?;
        let dy = value_f64(dy, "dy")?;
        let dz = value_f64(dz, "dz")?;
        require_positive("dx", dx)?;
        require_positive("dy", dy)?;
        require_positive("dz", dz)?;
        let at = parse_at(at)?;
        with_builder_mut(|store| {
            let id = store.builder.push(IrNode::Box { dx, dy, dz, at });
            Ok(id.0 as i32)
        })
    }

    /// Cylinder along +Z; base center at `at`.
    fn cylinder<'v>(
        radius: Value<'v>,
        height: Value<'v>,
        at: Option<Value<'v>>,
    ) -> anyhow::Result<i32> {
        let radius = value_f64(radius, "radius")?;
        let height = value_f64(height, "height")?;
        require_positive("radius", radius)?;
        require_positive("height", height)?;
        let at = parse_at(at)?;
        with_builder_mut(|store| {
            let id = store.builder.push(IrNode::Cylinder { radius, height, at });
            Ok(id.0 as i32)
        })
    }

    /// Sphere centered at `at`.
    fn sphere<'v>(radius: Value<'v>, at: Option<Value<'v>>) -> anyhow::Result<i32> {
        let radius = value_f64(radius, "radius")?;
        require_positive("radius", radius)?;
        let at = parse_at(at)?;
        with_builder_mut(|store| {
            let id = store.builder.push(IrNode::Sphere { radius, at });
            Ok(id.0 as i32)
        })
    }

    /// Cone along +Z; base radius at `at`, height toward +Z.
    fn cone<'v>(
        radius: Value<'v>,
        height: Value<'v>,
        at: Option<Value<'v>>,
    ) -> anyhow::Result<i32> {
        let radius = value_f64(radius, "radius")?;
        let height = value_f64(height, "height")?;
        require_positive("radius", radius)?;
        require_positive("height", height)?;
        let at = parse_at(at)?;
        with_builder_mut(|store| {
            let id = store.builder.push(IrNode::Cone { radius, height, at });
            Ok(id.0 as i32)
        })
    }

    /// Mirror through plane `xy` | `yz` | `zx` (through world origin).
    fn mirror<'v>(shape: Value<'v>, plane: &str) -> anyhow::Result<i32> {
        let of = shape_id(shape)?;
        let plane = plane.to_ascii_lowercase();
        if plane != "xy" && plane != "yz" && plane != "zx" && plane != "xz" {
            anyhow::bail!("mirror plane must be \"xy\", \"yz\", or \"zx\"");
        }
        let plane = if plane == "xz" {
            "zx".to_string()
        } else {
            plane
        };
        with_builder_mut(|store| {
            if store.builder.get(of).is_none() {
                anyhow::bail!("unknown shape id {}", of.0);
            }
            Ok(store.builder.push(IrNode::Mirror { of, plane }).0 as i32)
        })
    }

    /// Linear pattern: `count` copies of `shape` stepped by `(dx,dy,dz)` (includes original at 0).
    fn linear_pattern<'v>(
        shape: Value<'v>,
        count: Value<'v>,
        dx: Value<'v>,
        dy: Value<'v>,
        dz: Value<'v>,
    ) -> anyhow::Result<i32> {
        let base = shape_id(shape)?;
        let count = value_f64(count, "count")? as i32;
        if count < 1 {
            anyhow::bail!("linear_pattern count must be >= 1");
        }
        if count > 64 {
            anyhow::bail!("linear_pattern count must be <= 64");
        }
        let dx = value_f64(dx, "dx")?;
        let dy = value_f64(dy, "dy")?;
        let dz = value_f64(dz, "dz")?;
        with_builder_mut(|store| {
            if store.builder.get(base).is_none() {
                anyhow::bail!("unknown shape id {}", base.0);
            }
            let mut acc = base;
            for i in 1..count {
                let t = store.builder.push(IrNode::Translate {
                    of: base,
                    by: [dx * f64::from(i), dy * f64::from(i), dz * f64::from(i)],
                });
                acc = store.builder.push(IrNode::Boolean {
                    kind: BooleanKind::Union,
                    a: acc,
                    b: t,
                });
            }
            Ok(acc.0 as i32)
        })
    }

    /// Polar pattern about +Z through origin: `count` copies at equal angles (includes original).
    fn polar_pattern<'v>(shape: Value<'v>, count: Value<'v>) -> anyhow::Result<i32> {
        let base = shape_id(shape)?;
        let count = value_f64(count, "count")? as i32;
        if count < 1 {
            anyhow::bail!("polar_pattern count must be >= 1");
        }
        if count > 64 {
            anyhow::bail!("polar_pattern count must be <= 64");
        }
        with_builder_mut(|store| {
            if store.builder.get(base).is_none() {
                anyhow::bail!("unknown shape id {}", base.0);
            }
            let mut acc = base;
            if count == 1 {
                return Ok(acc.0 as i32);
            }
            let step = 360.0 / f64::from(count);
            for i in 1..count {
                let r = store.builder.push(IrNode::Rotate {
                    of: base,
                    axis: "z".into(),
                    deg: step * f64::from(i),
                });
                acc = store.builder.push(IrNode::Boolean {
                    kind: BooleanKind::Union,
                    a: acc,
                    b: r,
                });
            }
            Ok(acc.0 as i32)
        })
    }

    /// Boolean cut: `a` minus `b`.
    fn cut<'v>(a: Value<'v>, b: Value<'v>) -> anyhow::Result<i32> {
        boolean_op(BooleanKind::Cut, a, b)
    }

    /// Translate shape by mm offset `(dx, dy, dz)`.
    fn translate<'v>(
        shape: Value<'v>,
        dx: Value<'v>,
        dy: Value<'v>,
        dz: Value<'v>,
    ) -> anyhow::Result<i32> {
        let of = shape_id(shape)?;
        let dx = value_f64(dx, "dx")?;
        let dy = value_f64(dy, "dy")?;
        let dz = value_f64(dz, "dz")?;
        with_builder_mut(|store| {
            if store.builder.get(of).is_none() {
                anyhow::bail!("unknown shape id {}", of.0);
            }
            Ok(store
                .builder
                .push(IrNode::Translate {
                    of,
                    by: [dx, dy, dz],
                })
                .0 as i32)
        })
    }

    /// Rotate shape about world origin axis (`\"x\"`|`\"y\"`|`\"z\"`) by `deg` degrees.
    fn rotate<'v>(shape: Value<'v>, axis: &str, deg: Value<'v>) -> anyhow::Result<i32> {
        let of = shape_id(shape)?;
        let deg = value_f64(deg, "deg")?;
        let axis = axis.to_ascii_lowercase();
        if axis != "x" && axis != "y" && axis != "z" {
            anyhow::bail!("rotate axis must be \"x\", \"y\", or \"z\"");
        }
        with_builder_mut(|store| {
            if store.builder.get(of).is_none() {
                anyhow::bail!("unknown shape id {}", of.0);
            }
            Ok(store.builder.push(IrNode::Rotate { of, axis, deg }).0 as i32)
        })
    }

    /// Convenience: rotate about +Z.
    fn rotate_z<'v>(shape: Value<'v>, deg: Value<'v>) -> anyhow::Result<i32> {
        let of = shape_id(shape)?;
        let deg = value_f64(deg, "deg")?;
        with_builder_mut(|store| {
            if store.builder.get(of).is_none() {
                anyhow::bail!("unknown shape id {}", of.0);
            }
            Ok(store
                .builder
                .push(IrNode::Rotate {
                    of,
                    axis: "z".into(),
                    deg,
                })
                .0 as i32)
        })
    }

    /// Boolean union.
    fn union<'v>(a: Value<'v>, b: Value<'v>) -> anyhow::Result<i32> {
        boolean_op(BooleanKind::Union, a, b)
    }

    /// Boolean intersection.
    fn intersect<'v>(a: Value<'v>, b: Value<'v>) -> anyhow::Result<i32> {
        boolean_op(BooleanKind::Intersect, a, b)
    }

    /// Union of a list of shapes (left-fold).
    fn union_all<'v>(shapes: Value<'v>) -> anyhow::Result<i32> {
        let list = ListRef::from_value(shapes)
            .ok_or_else(|| anyhow::anyhow!("union_all expects a list of shape ids"))?;
        let mut iter = list.iter();
        let first = iter
            .next()
            .ok_or_else(|| anyhow::anyhow!("union_all: empty list"))?;
        let mut acc = shape_id(first)?;
        for s in iter {
            let b = shape_id(s)?;
            acc = with_builder_mut(|store| {
                if store.builder.get(acc).is_none() {
                    anyhow::bail!("unknown shape id {}", acc.0);
                }
                if store.builder.get(b).is_none() {
                    anyhow::bail!("unknown shape id {}", b.0);
                }
                Ok(store.builder.push(IrNode::Boolean {
                    kind: BooleanKind::Union,
                    a: acc,
                    b,
                }))
            })?;
        }
        Ok(acc.0 as i32)
    }

    /// Attach a product label; returns the labeled node id.
    fn solid<'v>(shape: Value<'v>, label: &str) -> anyhow::Result<i32> {
        let of = shape_id(shape)?;
        with_builder_mut(|store| {
            store.builder.label = Some(label.to_string());
            let id = store.builder.push(IrNode::Label {
                of,
                name: label.to_string(),
            });
            Ok(id.0 as i32)
        })
    }

    /// Fillet edges. Omit `edges` (or pass empty list) to fillet all edges.
    fn fillet<'v>(
        shape: Value<'v>,
        radius: Value<'v>,
        edges: Option<Value<'v>>,
    ) -> anyhow::Result<i32> {
        let of = shape_id(shape)?;
        let radius = value_f64(radius, "radius")?;
        require_positive("radius", radius)?;
        let edges = optional_edge_list(edges)?;
        with_builder_mut(|store| {
            if store.builder.get(of).is_none() {
                anyhow::bail!("unknown shape id {}", of.0);
            }
            let id = store.builder.push(IrNode::Fillet { of, radius, edges });
            Ok(id.0 as i32)
        })
    }

    /// Chamfer edges. Omit `edges` (or pass empty list) to chamfer all edges.
    fn chamfer<'v>(
        shape: Value<'v>,
        distance: Value<'v>,
        edges: Option<Value<'v>>,
    ) -> anyhow::Result<i32> {
        let of = shape_id(shape)?;
        let distance = value_f64(distance, "distance")?;
        require_positive("distance", distance)?;
        let edges = optional_edge_list(edges)?;
        with_builder_mut(|store| {
            if store.builder.get(of).is_none() {
                anyhow::bail!("unknown shape id {}", of.0);
            }
            let id = store.builder.push(IrNode::Chamfer {
                of,
                distance,
                edges,
            });
            Ok(id.0 as i32)
        })
    }

    /// Discard print (hermetic).
    fn print<'v>(msg: Value<'v>) -> anyhow::Result<NoneType> {
        let _ = msg;
        Ok(NoneType)
    }
}

fn optional_edge_list(edges: Option<Value<'_>>) -> anyhow::Result<Vec<u32>> {
    let Some(v) = edges else {
        return Ok(Vec::new());
    };
    let list = ListRef::from_value(v)
        .ok_or_else(|| anyhow::anyhow!("edges= expects a list of edge indices"))?;
    let mut out = Vec::new();
    for item in list.iter() {
        let i = item
            .unpack_i32()
            .ok_or_else(|| anyhow::anyhow!("edge index must be int"))?;
        if i < 0 {
            anyhow::bail!("edge index must be >= 0, got {i}");
        }
        out.push(i as u32);
    }
    Ok(out)
}

fn boolean_op<'v>(kind: BooleanKind, a: Value<'v>, b: Value<'v>) -> anyhow::Result<i32> {
    let a = shape_id(a)?;
    let b = shape_id(b)?;
    with_builder_mut(|store| {
        if store.builder.get(a).is_none() {
            anyhow::bail!("unknown shape id {}", a.0);
        }
        if store.builder.get(b).is_none() {
            anyhow::bail!("unknown shape id {}", b.0);
        }
        let id = store.builder.push(IrNode::Boolean { kind, a, b });
        Ok(id.0 as i32)
    })
}
