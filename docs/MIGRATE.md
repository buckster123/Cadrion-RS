# Migrator — build123d → Cadre skeleton (H8 + H2-7 + H3-9)

## Scope

Best-effort **structure + params** only. Not full semantic parity with build123d.

Input is treated as untrusted text. Refuse: `exec`/`eval`/`subprocess`/`open`/`getattr`/…

Shaped from **public** build123d-style APIs only — never third-party private sources.

## CLI

```sh
cargo run -p cadre-cli -- migrate fixtures/migrate/01_simple_box.py --json
cargo run -p cadre-cli -- migrate fixtures/migrate/06_circle_extrude.py --json
```

## Coverage

| Pattern | Cadre output |
|---------|----------------|
| `Box` / kwargs | `box(...)` |
| `Cylinder` / `Sphere` / `Cone` | matching stdlib |
| `Rectangle` + `extrude(amount)` | `box(w,d,h)` |
| `Circle` + `extrude(amount)` | `cylinder(r,h)` |
| `Locations((x,y,z))` / `Location` | `translate(shape, x,y,z)` (order-paired) |
| `fillet` / `chamfer` | **note + TODO comment** (not fake-applied on mock) |
| `-=` / `Mode.SUBTRACT` | sequential `cut` |
| multiple solids | sequential `union` |

## Fixtures

| File | Intent |
|------|--------|
| `fixtures/migrate/01_simple_box.py` | Box + params |
| `fixtures/migrate/02_plate_hole.py` | Box + Cylinder |
| `fixtures/migrate/03_kwargs_sphere.py` | kwargs Box + Sphere |
| `fixtures/migrate/04_locations_offset.py` | **H2-7** Locations → translate |
| `fixtures/migrate/05_fillet_extrude.py` | **H2-7** extrude + fillet note |
| `fixtures/migrate/06_circle_extrude.py` | **H3-9** Circle+extrude → cylinder |

## Honesty

- Workplanes / full Mode stack / face selections not reconstructed
- Fillet/chamfer are **stubs** until OCCT review
- Always review the skeleton before fab
- Unsafe Python still refused (`exec`/`eval`/`getattr`/`globals`/…)
