# cadrion-lang

Hermetic Starlark host for Cadrion-RS. Evaluates `.cad.star` sources to a feature IR document
that `cadrion-kernel` backends will execute.

```rust
use cadrion_lang::{evaluate, EvalOptions};

let src = r#"
P = params(width=100.0, depth=60.0, height=20.0)
def gen_step():
    return solid(box(P.width, P.depth, P.height, at=CENTER), label="block")
"#;
let r = evaluate(src, &EvalOptions::new("block.cad.star").with_override("width", 120.0));
assert!(r.ok);
```

See `docs/design.md` (Feature IR) and crate docs.
