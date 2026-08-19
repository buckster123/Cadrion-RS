//! Clean-room build123d → Cadrion `.cad.star` skeleton migrator (H8).
//!
//! Best-effort structure + params only — **not** full semantic parity.
//! Input is treated as untrusted text; refuse obvious unsafe patterns.
//! Shaped from **public** build123d-style APIs only (no third-party repo source).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

const MAX_BYTES: usize = 512_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateReport {
    pub ok: bool,
    /// True when input was refused (unsafe / empty / binary).
    pub refused: bool,
    pub skeleton: String,
    pub notes: Vec<String>,
    pub params: BTreeMap<String, f64>,
    pub solids: Vec<String>,
    pub source_hint: String,
}

/// Migrate public-API-shaped build123d Python text → Cadrion skeleton.
pub fn migrate_build123d_skeleton(source: &str) -> MigrateReport {
    let mut notes = Vec::new();
    if source.len() > MAX_BYTES {
        return refuse("source too large (>512 KiB)", notes);
    }
    if source.bytes().any(|b| b == 0) {
        return refuse("binary/null bytes in source", notes);
    }
    if let Some(why) = unsafe_reason(source) {
        return refuse(why, notes);
    }
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return refuse("empty source", notes);
    }

    // Soft signals this looks like CAD Python
    let looks_b123 = source.contains("build123d")
        || source.contains("BuildPart")
        || source.contains("Box(")
        || source.contains("Cylinder(")
        || source.contains("Sphere(")
        || source.contains("from ocp")
        || source.contains("import cadquery")
        || source.contains("cq.");
    if !looks_b123 {
        notes.push(
            "no strong build123d/Box/Cylinder markers — still emitting best-effort skeleton".into(),
        );
    }

    let params = extract_params(source);
    let mut solids = Vec::new();
    let mut body_lines = Vec::new();

    // Box(dx, dy, dz) or Box(length=, width=, height=)
    for cap in box_re().captures_iter(source) {
        if let Some((dx, dy, dz, label)) = parse_box_cap(&cap, &params) {
            let name = unique_name("box", solids.len());
            body_lines.push(format!(
                "    {name} = box({dx}, {dy}, {dz}, at=CENTER)  # {label}"
            ));
            solids.push(name);
        }
    }
    for cap in cyl_re().captures_iter(source) {
        if let Some((r, h, label)) = parse_cyl_cap(&cap, &params) {
            let name = unique_name("cyl", solids.len());
            body_lines.push(format!(
                "    {name} = cylinder({r}, {h}, at=CENTER)  # {label}"
            ));
            solids.push(name);
        }
    }
    for cap in sphere_re().captures_iter(source) {
        if let Some((r, label)) = parse_sphere_cap(&cap, &params) {
            let name = unique_name("sph", solids.len());
            body_lines.push(format!("    {name} = sphere({r}, at=CENTER)  # {label}"));
            solids.push(name);
        }
    }

    // H3-9: Circle + extrude(amount) → cylinder(r, h)
    for (r, h, label) in extract_extrude_cylinders(source, &params) {
        let name = unique_name("excyl", solids.len());
        body_lines.push(format!(
            "    {name} = cylinder({r}, {h}, at=CENTER)  # {label}"
        ));
        solids.push(name);
        notes.push("Circle+extrude → cylinder (axis assumed +Z)".into());
    }

    // H2-7: Rectangle + extrude(amount) → box(w, d, amount)
    // Skip default-footprint extrude if we already consumed extrude as Circle.
    let skip_bare_extrude = source.contains("Circle(");
    for (w, d, h, label) in extract_extrude_boxes(source, &params, skip_bare_extrude) {
        let name = unique_name("ext", solids.len());
        body_lines.push(format!(
            "    {name} = box({w}, {d}, {h}, at=CENTER)  # {label}"
        ));
        solids.push(name);
        notes.push("extrude/Rectangle → box (workplane normal assumed +Z)".into());
    }

    // H2-7: Cone if present
    for cap in cone_re().captures_iter(source) {
        if let Some((r, h, label)) = parse_cone_cap(&cap, &params) {
            let name = unique_name("cone", solids.len());
            body_lines.push(format!("    {name} = cone({r}, {h}, at=CENTER)  # {label}"));
            solids.push(name);
        }
    }

    if solids.is_empty() {
        notes.push("no Box/Cylinder/Sphere/extrude calls recovered — placeholder box".into());
        body_lines.push("    body = box(10.0, 10.0, 10.0, at=CENTER)  # placeholder".into());
        solids.push("body".into());
    }

    // H2-7: Locations / Location offsets → translate on solids after the base.
    // Heuristic: if more solids than locations, pair locs with trailing solids
    // (common: base then Locations{ feature }).
    let locs = extract_locations(source, &params);
    if !locs.is_empty() {
        let start = if solids.len() > locs.len() {
            solids.len() - locs.len()
        } else {
            0
        };
        notes.push(format!(
            "Locations/Location → translate on {} solid(s) (from index {start})",
            locs.len().min(solids.len().saturating_sub(start))
        ));
        let n = locs.len().min(solids.len().saturating_sub(start));
        for (i, &(x, y, z)) in locs.iter().take(n).enumerate() {
            if x.abs() < 1e-12 && y.abs() < 1e-12 && z.abs() < 1e-12 {
                continue;
            }
            let si = start + i;
            let src = solids[si].clone();
            let name = unique_name("tr", body_lines.len());
            body_lines.push(format!(
                "    {name} = translate({src}, {}, {}, {})  # from Locations",
                fmt_num(x),
                fmt_num(y),
                fmt_num(z)
            ));
            solids[si] = name;
        }
    }

    // H2-7: fillet / chamfer → notes + comment stub (do not fake mock goldens)
    if fillet_re().is_match(source) {
        notes.push(
            "Fillet(...) seen — not auto-applied (mock Unsupported; use OCCT after review)".into(),
        );
        body_lines.push(
            "    # TODO fillet: e.g. body = fillet(body, radius)  # requires --kernel occt".into(),
        );
    }
    if chamfer_re().is_match(source) {
        notes.push(
            "Chamfer(...) seen — not auto-applied (mock Unsupported; use OCCT after review)".into(),
        );
        body_lines.push(
            "    # TODO chamfer: e.g. body = chamfer(body, distance)  # requires --kernel occt"
                .into(),
        );
    }

    // Combine: if source has subtract/cut markers, cut later solids from first
    let has_cut = source.contains("-=")
        || source.contains(".cut(")
        || source.contains("mode=Mode.SUBTRACT")
        || source.contains("Mode.SUBTRACT");
    let root = if solids.len() == 1 {
        solids[0].clone()
    } else if has_cut && solids.len() >= 2 {
        notes
            .push("detected subtract-ish ops → sequential cut() of later solids from first".into());
        let mut expr = solids[0].clone();
        for s in &solids[1..] {
            let tmp = unique_name("cut", body_lines.len());
            body_lines.push(format!("    {tmp} = cut({expr}, {s})"));
            expr = tmp;
        }
        expr
    } else {
        notes.push("multiple solids → sequential union() (topology may differ)".into());
        let mut expr = solids[0].clone();
        for s in &solids[1..] {
            let tmp = unique_name("u", body_lines.len());
            body_lines.push(format!("    {tmp} = union({expr}, {s})"));
            expr = tmp;
        }
        expr
    };

    let mut sk = String::new();
    sk.push_str("# Cadrion skeleton migrated from build123d-style Python (H8/H2-7 clean-room).\n");
    sk.push_str("# Best-effort structure + params — NOT full semantic parity.\n");
    sk.push_str("# Review numbers, placements, and booleans before fab.\n\n");

    if params.is_empty() {
        sk.push_str("P = params()\n\n");
    } else {
        sk.push_str("P = params(\n");
        for (k, v) in &params {
            sk.push_str(&format!("    {k}={v},\n"));
        }
        sk.push_str(")\n\n");
    }

    sk.push_str("def gen_step():\n");
    for line in &body_lines {
        sk.push_str(line);
        sk.push('\n');
    }
    sk.push_str(&format!("    return solid({root}, label=\"migrated\")\n"));

    // Validate skeleton evaluates
    let eval = crate::evaluate(&sk, &crate::EvalOptions::new("migrated.cad.star"));
    if !eval.ok {
        notes.push(format!(
            "generated skeleton failed eval: {:?}",
            eval.diagnostics
        ));
        return MigrateReport {
            ok: false,
            refused: false,
            skeleton: sk,
            notes,
            params,
            solids,
            source_hint: "build123d-style".into(),
        };
    }
    notes.push(format!(
        "eval ok · {} param(s) · {} solid op(s)",
        params.len(),
        solids.len()
    ));
    MigrateReport {
        ok: true,
        refused: false,
        skeleton: sk,
        notes,
        params,
        solids,
        source_hint: if looks_b123 {
            "build123d-style".into()
        } else {
            "generic-python-cad".into()
        },
    }
}

fn refuse(why: &str, mut notes: Vec<String>) -> MigrateReport {
    notes.push(why.into());
    MigrateReport {
        ok: false,
        refused: true,
        skeleton: String::new(),
        notes,
        params: BTreeMap::new(),
        solids: vec![],
        source_hint: "refused".into(),
    }
}

fn unsafe_reason(source: &str) -> Option<&'static str> {
    let lower = source.to_ascii_lowercase();
    let needles = [
        "os.system",
        "subprocess",
        "socket.",
        "__import__",
        "importlib",
        "eval(",
        "exec(",
        "compile(",
        "pty.",
        "ctypes",
        "pickle",
        "marshal",
        "pathlib.path(", // weak
        "open(",
        "write(",
        "urllib",
        "requests.",
        "http.client",
        "shutil.rmtree",
        "rmtree",
        "getattr(",
        "setattr(",
        "globals(",
        "locals(",
        "builtins",
        "breakpoint(",
        "input(",
    ];
    for n in needles {
        if lower.contains(n) {
            return Some("refused: potentially unsafe Python pattern");
        }
    }
    None
}

fn extract_params(source: &str) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    // name = 12.3 or name = 12
    let re = assign_re();
    for cap in re.captures_iter(source) {
        let name = cap.get(1).unwrap().as_str();
        if name.starts_with('_') || name == "i" || name == "j" || name == "k" {
            continue;
        }
        if matches!(
            name,
            "True" | "False" | "None" | "mode" | "align" | "until" | "amount"
        ) {
            continue;
        }
        if let Ok(v) = cap.get(2).unwrap().as_str().parse::<f64>() {
            if v.is_finite() {
                out.insert(sanitize_ident(name), v);
            }
        }
    }
    out
}

fn sanitize_ident(s: &str) -> String {
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_alphanumeric() || c == '_' {
            if i == 0 && c.is_ascii_digit() {
                o.push('_');
            }
            o.push(c);
        } else {
            o.push('_');
        }
    }
    if o.is_empty() {
        "p".into()
    } else {
        o
    }
}

fn unique_name(prefix: &str, i: usize) -> String {
    format!("{prefix}{i}")
}

fn resolve_num(tok: &str, params: &BTreeMap<String, f64>) -> Option<f64> {
    let t = tok.trim();
    if let Ok(v) = t.parse::<f64>() {
        return Some(v);
    }
    // P.width style
    if let Some(rest) = t.strip_prefix("P.") {
        return params.get(rest).copied();
    }
    params.get(t).copied()
}

fn parse_box_cap(
    cap: &regex::Captures<'_>,
    params: &BTreeMap<String, f64>,
) -> Option<(String, String, String, String)> {
    // Either positional g1,g2,g3 or kwargs
    if let (Some(a), Some(b), Some(c)) = (cap.name("a"), cap.name("b"), cap.name("c")) {
        let dx = fmt_num(resolve_num(a.as_str(), params)?);
        let dy = fmt_num(resolve_num(b.as_str(), params)?);
        let dz = fmt_num(resolve_num(c.as_str(), params)?);
        return Some((dx, dy, dz, "Box positional".into()));
    }
    let body = cap.name("body")?.as_str();
    let l = kw_num(body, &["length", "l", "x", "dx"], params);
    let w = kw_num(body, &["width", "w", "y", "dy"], params);
    let h = kw_num(body, &["height", "h", "z", "dz"], params);
    match (l, w, h) {
        (Some(l), Some(w), Some(h)) => {
            Some((fmt_num(l), fmt_num(w), fmt_num(h), "Box kwargs".into()))
        }
        _ => None,
    }
}

fn parse_cyl_cap(
    cap: &regex::Captures<'_>,
    params: &BTreeMap<String, f64>,
) -> Option<(String, String, String)> {
    if let (Some(a), Some(b)) = (cap.name("a"), cap.name("b")) {
        // Cylinder(radius, height) OR Cylinder(height, radius) — build123d often radius first
        let r = resolve_num(a.as_str(), params)?;
        let h = resolve_num(b.as_str(), params)?;
        return Some((fmt_num(r), fmt_num(h), "Cylinder positional".into()));
    }
    let body = cap.name("body")?.as_str();
    let r = kw_num(body, &["radius", "r"], params);
    let h = kw_num(body, &["height", "h"], params);
    match (r, h) {
        (Some(r), Some(h)) => Some((fmt_num(r), fmt_num(h), "Cylinder kwargs".into())),
        _ => None,
    }
}

fn parse_sphere_cap(
    cap: &regex::Captures<'_>,
    params: &BTreeMap<String, f64>,
) -> Option<(String, String)> {
    if let Some(a) = cap.name("a") {
        let r = resolve_num(a.as_str(), params)?;
        return Some((fmt_num(r), "Sphere positional".into()));
    }
    let body = cap.name("body")?.as_str();
    let r = kw_num(body, &["radius", "r"], params)?;
    Some((fmt_num(r), "Sphere kwargs".into()))
}

fn kw_num(body: &str, keys: &[&str], params: &BTreeMap<String, f64>) -> Option<f64> {
    for k in keys {
        let re = Regex::new(&format!(
            r"(?i)\b{k}\s*=\s*([A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)"
        ))
        .ok()?;
        if let Some(c) = re.captures(body) {
            return resolve_num(c.get(1)?.as_str(), params);
        }
    }
    None
}

fn fmt_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-12 {
        format!("{:.1}", v.round())
    } else {
        format!("{v}")
    }
}

/// H2-7: Locations((x,y,z)) / Location((x,y,z)) / Locations((x,y))
fn extract_locations(source: &str, params: &BTreeMap<String, f64>) -> Vec<(f64, f64, f64)> {
    let mut out = Vec::new();
    let re = loc_re();
    for cap in re.captures_iter(source) {
        let a = cap.name("a").map(|m| m.as_str());
        let b = cap.name("b").map(|m| m.as_str());
        let c = cap.name("c").map(|m| m.as_str());
        let (Some(a), Some(b)) = (a, b) else {
            continue;
        };
        let x = match resolve_num(a, params) {
            Some(v) => v,
            None => continue,
        };
        let y = match resolve_num(b, params) {
            Some(v) => v,
            None => continue,
        };
        let z = c.and_then(|t| resolve_num(t, params)).unwrap_or(0.0);
        out.push((x, y, z));
    }
    out
}

/// Rectangle(w,h) + extrude(amount) → boxes. Also bare extrude(amount) with default footprint.
fn extract_extrude_boxes(
    source: &str,
    params: &BTreeMap<String, f64>,
    skip_bare_extrude: bool,
) -> Vec<(String, String, String, String)> {
    let mut out = Vec::new();
    // collect rectangles
    let mut rects: Vec<(f64, f64)> = Vec::new();
    for cap in rect_re().captures_iter(source) {
        if let (Some(a), Some(b)) = (cap.name("a"), cap.name("b")) {
            if let (Some(w), Some(d)) = (
                resolve_num(a.as_str(), params),
                resolve_num(b.as_str(), params),
            ) {
                rects.push((w, d));
            }
        } else if let Some(body) = cap.name("body") {
            let w = kw_num(body.as_str(), &["width", "w", "x"], params);
            let d = kw_num(body.as_str(), &["height", "h", "y", "length", "l"], params);
            if let (Some(w), Some(d)) = (w, d) {
                rects.push((w, d));
            }
        }
    }
    let mut amounts: Vec<f64> = Vec::new();
    for cap in extrude_re().captures_iter(source) {
        if let Some(a) = cap.name("a") {
            if let Some(v) = resolve_num(a.as_str(), params) {
                amounts.push(v.abs().max(1e-9));
            }
        } else if let Some(body) = cap.name("body") {
            if let Some(v) = kw_num(body.as_str(), &["amount", "height", "h", "depth"], params) {
                amounts.push(v.abs().max(1e-9));
            }
        }
    }
    if amounts.is_empty() {
        return out;
    }
    if rects.is_empty() {
        if skip_bare_extrude {
            return out;
        }
        // extrude without rectangle — default 20x20 footprint once per amount
        for h in amounts {
            out.push((
                "20.0".into(),
                "20.0".into(),
                fmt_num(h),
                "extrude (default footprint)".into(),
            ));
        }
        return out;
    }
    let n = amounts.len().max(rects.len());
    for i in 0..n {
        let (w, d) = rects[i.min(rects.len() - 1)];
        let h = amounts[i.min(amounts.len() - 1)];
        out.push((
            fmt_num(w),
            fmt_num(d),
            fmt_num(h),
            "Rectangle+extrude".into(),
        ));
    }
    out
}

/// Circle(r) + extrude(amount) → cylinders.
fn extract_extrude_cylinders(
    source: &str,
    params: &BTreeMap<String, f64>,
) -> Vec<(String, String, String)> {
    let mut circles: Vec<f64> = Vec::new();
    for cap in circle_re().captures_iter(source) {
        if let Some(a) = cap.name("a") {
            if let Some(r) = resolve_num(a.as_str(), params) {
                circles.push(r.abs().max(1e-9));
            }
        } else if let Some(body) = cap.name("body") {
            if let Some(r) = kw_num(body.as_str(), &["radius", "r"], params) {
                circles.push(r.abs().max(1e-9));
            }
        }
    }
    if circles.is_empty() {
        return Vec::new();
    }
    let mut amounts: Vec<f64> = Vec::new();
    for cap in extrude_re().captures_iter(source) {
        if let Some(a) = cap.name("a") {
            if let Some(v) = resolve_num(a.as_str(), params) {
                amounts.push(v.abs().max(1e-9));
            }
        } else if let Some(body) = cap.name("body") {
            if let Some(v) = kw_num(body.as_str(), &["amount", "height", "h", "depth"], params) {
                amounts.push(v.abs().max(1e-9));
            }
        }
    }
    if amounts.is_empty() {
        return Vec::new();
    }
    let n = amounts.len().max(circles.len());
    let mut out = Vec::new();
    for i in 0..n {
        let r = circles[i.min(circles.len() - 1)];
        let h = amounts[i.min(amounts.len() - 1)];
        out.push((fmt_num(r), fmt_num(h), "Circle+extrude".into()));
    }
    out
}

fn parse_cone_cap(
    cap: &regex::Captures<'_>,
    params: &BTreeMap<String, f64>,
) -> Option<(String, String, String)> {
    if let (Some(a), Some(b)) = (cap.name("a"), cap.name("b")) {
        // Cone(radius, height) — ignore optional third
        let r = resolve_num(a.as_str(), params)?;
        let h = resolve_num(b.as_str(), params)?;
        return Some((fmt_num(r), fmt_num(h), "Cone positional".into()));
    }
    let body = cap.name("body")?.as_str();
    let r = kw_num(body, &["radius", "r", "bottom_radius"], params)?;
    let h = kw_num(body, &["height", "h"], params)?;
    Some((fmt_num(r), fmt_num(h), "Cone kwargs".into()))
}

fn box_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \bBox\s*\(
              (?:
                \s*(?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)\s*,
                \s*(?P<b>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)\s*,
                \s*(?P<c>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
              |
                (?P<body>[^)]*)
              )
            \)",
        )
        .unwrap()
    })
}

fn cyl_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \bCylinder\s*\(
              (?:
                \s*(?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)\s*,
                \s*(?P<b>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
              |
                (?P<body>[^)]*)
              )
            \)",
        )
        .unwrap()
    })
}

fn sphere_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \bSphere\s*\(
              (?:
                \s*(?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
              |
                (?P<body>[^)]*)
              )
            \)",
        )
        .unwrap()
    })
}

fn cone_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \bCone\s*\(
              (?:
                \s*(?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)\s*,
                \s*(?P<b>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
                (?:\s*,\s*(?P<c>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?))?
              |
                (?P<body>[^)]*)
              )
            \)",
        )
        .unwrap()
    })
}

fn circle_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \bCircle\s*\(
              (?:
                \s*(?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
              |
                (?P<body>[^)]*)
              )
            \)",
        )
        .unwrap()
    })
}

fn rect_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \bRectangle\s*\(
              (?:
                \s*(?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)\s*,
                \s*(?P<b>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
              |
                (?P<body>[^)]*)
              )
            \)",
        )
        .unwrap()
    })
}

fn extrude_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?x)
            \bextrude\s*\(
              (?:
                \s*(?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
              |
                (?P<body>[^)]*)
              )
            \)",
        )
        .unwrap()
    })
}

fn loc_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // Locations((x, y, z)) or Location((x, y)) or Locations((x, y))
        Regex::new(
            r"(?x)
            \bLocations?\s*\(\s*\(\s*
              (?P<a>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)\s*,\s*
              (?P<b>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?)
              (?:\s*,\s*(?P<c>[A-Za-z_][\w.]*|-?\d+(?:\.\d+)?))?
            \s*\)",
        )
        .unwrap()
    })
}

fn fillet_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bfillet\s*\(").unwrap())
}

fn chamfer_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bchamfer\s*\(").unwrap())
}

fn assign_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?m)^\s*([A-Za-z_][\w]*)\s*=\s*(-?\d+(?:\.\d+)?)\s*(?:#.*)?$").unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn migrates_simple_box_cylinder() {
        let src = r#"
from build123d import *

length = 80.0
width = 40.0
height = 12.0
hole_r = 5.0

with BuildPart() as part:
    Box(length, width, height)
    Cylinder(hole_r, height)
"#;
        let r = migrate_build123d_skeleton(src);
        assert!(r.ok, "{r:?}");
        assert!(!r.refused);
        assert!(r.skeleton.contains("box("));
        assert!(r.skeleton.contains("cylinder("));
        assert!(r.params.contains_key("length"));
    }

    #[test]
    fn refuses_exec() {
        let r = migrate_build123d_skeleton("exec('os.system(\"rm -rf /\")')\nBox(1,2,3)\n");
        assert!(r.refused);
        assert!(!r.ok);
    }

    #[test]
    fn kwargs_box() {
        let src = "from build123d import *\nBox(length=10, width=20, height=30)\n";
        let r = migrate_build123d_skeleton(src);
        assert!(r.ok, "{r:?}");
        assert!(r.skeleton.contains("10.0") || r.skeleton.contains("box(10"));
    }

    #[test]
    fn locations_become_translate() {
        let src = r#"
from build123d import *
with BuildPart() as p:
    Box(10, 10, 5)
    with Locations((20, 0, 0)):
        Cylinder(3, 8)
"#;
        let r = migrate_build123d_skeleton(src);
        assert!(r.ok, "{r:?}");
        assert!(
            r.skeleton.contains("translate("),
            "expected translate: {}",
            r.skeleton
        );
        assert!(r.notes.iter().any(|n| n.contains("Locations")));
    }

    #[test]
    fn extrude_rectangle_to_box() {
        let src = r#"
from build123d import *
with BuildSketch() as s:
    Rectangle(40, 20)
with BuildPart() as p:
    extrude(amount=12)
"#;
        let r = migrate_build123d_skeleton(src);
        assert!(r.ok, "{r:?}");
        assert!(r.skeleton.contains("box("));
        assert!(r.notes.iter().any(|n| n.contains("extrude")));
    }

    #[test]
    fn circle_extrude_to_cylinder() {
        let src = r#"
from build123d import *
r = 8.0
h = 24.0
with BuildSketch() as s:
    Circle(r)
with BuildPart() as p:
    extrude(amount=h)
"#;
        let r = migrate_build123d_skeleton(src);
        assert!(r.ok, "{r:?}");
        assert!(r.skeleton.contains("cylinder("), "{}", r.skeleton);
        assert!(r.notes.iter().any(|n| n.contains("Circle")));
    }

    #[test]
    fn refuses_getattr() {
        let r = migrate_build123d_skeleton("getattr(__builtins__, 'eval')('1')\nBox(1,2,3)\n");
        assert!(r.refused);
    }

    #[test]
    fn fillet_notes_not_applied() {
        let src = r#"
from build123d import *
with BuildPart() as p:
    Box(30, 20, 10)
    fillet(2.0)
"#;
        let r = migrate_build123d_skeleton(src);
        assert!(r.ok, "{r:?}");
        assert!(r.skeleton.contains("TODO fillet") || r.notes.iter().any(|n| n.contains("Fillet")));
        // must still eval (comment only)
        assert!(r.ok);
    }

    #[test]
    fn golden_fixtures_eval() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/migrate");
        for name in [
            "01_simple_box.py",
            "02_plate_hole.py",
            "03_kwargs_sphere.py",
            "04_locations_offset.py",
            "05_fillet_extrude.py",
            "06_circle_extrude.py",
        ] {
            let p = root.join(name);
            let src = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
            let r = migrate_build123d_skeleton(&src);
            assert!(r.ok && !r.refused, "{name}: {r:?}");
            assert!(r.skeleton.contains("def gen_step"));
        }
    }
}
