//! `cadrion view` — loopback HTTP viewer: snaps + G-code scrub + URDF jog (alpha).

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::cli::{Cli, ViewArgs};
use crate::output::{emit, ExitCode};

pub fn run(cli: &Cli, args: &ViewArgs) -> ExitCode {
    if args.paths.is_empty() {
        let v = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-USAGE", "message": "pass at least one path"}]});
        emit(cli.json, &v, false);
        return ExitCode::Usage;
    }

    let mut entries = Vec::new();
    for p in &args.paths {
        match prepare_entry(p) {
            Ok(e) => entries.push(e),
            Err(msg) => {
                let v = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-VIEW", "message": msg}]});
                emit(cli.json, &v, false);
                return ExitCode::Io;
            }
        }
    }

    if args.once {
        let links: Vec<_> = entries
            .iter()
            .map(|e| {
                json!({
                    "path": e.path,
                    "root": e.root,
                    "kind": e.kind,
                    "meta": e.meta_name,
                    "url": null,
                    "note": "prepared; run without --once to serve",
                })
            })
            .collect();
        let body = json!({
            "ok": true,
            "once": true,
            "links": links,
        });
        emit(cli.json, &body, true);
        return ExitCode::Ok;
    }

    let listener = match bind_viewer(&args.host, args.port) {
        Ok(l) => l,
        Err(e) => {
            let v = json!({"ok": false, "diagnostics": [{"code": "CADRION-E-IO", "message": e}]});
            emit(cli.json, &v, false);
            return ExitCode::Io;
        }
    };
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{}:{}", addr.ip(), addr.port());

    let links: Vec<_> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            json!({
                "path": e.path,
                "url": format!("{base}/v/{i}/"),
                "kind": e.kind,
                "meta": e.meta_name,
            })
        })
        .collect();

    let body = json!({
        "ok": true,
        "base": base,
        "links": links,
        "note": "viewer H3-5 — mesh 3D + PMI dim overlay / gcode scrub / robot 3D jog; Ctrl-C to stop",
    });
    emit(cli.json, &body, true);
    if !cli.json && !cli.quiet {
        eprintln!("cadrion view listening on {base}");
        for l in &links {
            if let Some(u) = l.get("url").and_then(|u| u.as_str()) {
                eprintln!("  {u}");
            }
        }
    }

    let entries = std::sync::Arc::new(entries);
    let base_c = base.clone();
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let ents = entries.clone();
                let base = base_c.clone();
                thread::spawn(move || {
                    let _ = handle_client(&mut stream, &ents, &base);
                });
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

fn bind_viewer(host: &str, port: u16) -> Result<TcpListener, String> {
    let bind = format!("{host}:{port}");
    match TcpListener::bind(&bind) {
        Ok(l) => Ok(l),
        Err(e) => {
            for d in 1..30 {
                let b = format!("{host}:{}", port + d);
                if let Ok(l) = TcpListener::bind(&b) {
                    return Ok(l);
                }
            }
            Err(format!("bind {bind}: {e}"))
        }
    }
}

struct Entry {
    path: PathBuf,
    kind: &'static str,
    /// Directory served under /v/i/…
    root: PathBuf,
    /// Optional primary meta file name (path.json / robot.json).
    meta_name: Option<String>,
}

fn prepare_entry(p: &Path) -> Result<Entry, String> {
    let p = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    if p.is_dir() {
        return Ok(Entry {
            path: p.clone(),
            kind: "snap",
            root: p,
            meta_name: None,
        });
    }
    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let lower = name.to_ascii_lowercase();

    if lower.ends_with(".cad.star") || lower.ends_with(".star") {
        let stem = name
            .strip_suffix(".cad.star")
            .or_else(|| name.strip_suffix(".star"))
            .unwrap_or(name);
        let snap = p.with_file_name(format!("{stem}.snap"));
        let src = fs::read_to_string(&p).map_err(|e| e.to_string())?;
        let eval = cadrion_lang::evaluate(&src, &cadrion_lang::EvalOptions::new(name));
        if !eval.ok {
            return Err(format!("eval failed: {:?}", eval.diagnostics));
        }
        let (mesh, notes) = cadrion_render::mesh_from_ir(eval.ir.as_ref().unwrap())?;
        let opts = cadrion_render::SnapshotOptions {
            notes,
            width: 384,
            height: 384,
            gif_frames: 16,
            ..Default::default()
        };
        cadrion_render::write_snapshot_packet(&mesh, &snap, &opts)?;
        // H2-6: coarse triangle mesh for interactive canvas 3D
        write_mesh_json(&snap.join("mesh.json"), &mesh)?;
        // H3-5: PMI drawing packet for canvas overlay (sidecar or auto-dims)
        write_drawing_for_view(&p, &snap, eval.ir.as_ref().unwrap())?;
        return Ok(Entry {
            path: p,
            kind: "star",
            root: snap,
            meta_name: Some("mesh.json".into()),
        });
    }

    if lower.ends_with(".gcode") || lower.ends_with(".gco") || lower.ends_with(".nc") {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("gcode");
        let root = p.with_file_name(format!("{stem}.view"));
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let text = fs::read_to_string(&p).map_err(|e| e.to_string())?;
        let path = cadrion_fab::extract_gcode_path(&text);
        let meta = root.join("path.json");
        fs::write(
            &meta,
            serde_json::to_string_pretty(&path).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        // Keep a tiny copy of source head for display
        let head: String = text.lines().take(40).collect::<Vec<_>>().join("\n");
        fs::write(root.join("preview.txt"), head).ok();
        return Ok(Entry {
            path: p,
            kind: "gcode",
            root,
            meta_name: Some("path.json".into()),
        });
    }

    if lower.ends_with(".robot.json") || lower.ends_with(".urdf") || lower.ends_with(".json") {
        let robot = load_robot(&p)?;
        let payload = cadrion_robot::jog_payload(&robot);
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("robot");
        let root = p.with_file_name(format!("{stem}.view"));
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let meta = root.join("robot.json");
        fs::write(
            &meta,
            serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        return Ok(Entry {
            path: p,
            kind: "robot",
            root,
            meta_name: Some("robot.json".into()),
        });
    }

    if lower.ends_with(".png") || lower.ends_with(".gif") {
        let root = p.parent().unwrap_or(Path::new(".")).to_path_buf();
        return Ok(Entry {
            path: p,
            kind: "image",
            root,
            meta_name: None,
        });
    }

    Err(format!(
        "unsupported view target: {} (want .snap / .cad.star / .gcode / .robot.json / .urdf)",
        p.display()
    ))
}

fn load_robot(p: &Path) -> Result<cadrion_robot::RobotSpec, String> {
    let text = fs::read_to_string(p).map_err(|e| e.to_string())?;
    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if name.ends_with(".urdf") {
        // Alpha: jog payload needs RobotSpec JSON. Validate URDF then refuse with hint.
        cadrion_robot::parse_urdf_xml(&text).map_err(|e| e.to_string())?;
        return Err(
            "URDF jog alpha expects Cadrion .robot.json (export via `cadrion robot emit`). URDF validated OK."
                .into(),
        );
    }
    serde_json::from_str(&text).map_err(|e| format!("robot json: {e}"))
}

fn write_mesh_json(path: &Path, mesh: &cadrion_kernel::Mesh) -> Result<(), String> {
    let mut xmin = f32::INFINITY;
    let mut ymin = f32::INFINITY;
    let mut zmin = f32::INFINITY;
    let mut xmax = f32::NEG_INFINITY;
    let mut ymax = f32::NEG_INFINITY;
    let mut zmax = f32::NEG_INFINITY;
    for c in mesh.positions.chunks_exact(3) {
        xmin = xmin.min(c[0]);
        ymin = ymin.min(c[1]);
        zmin = zmin.min(c[2]);
        xmax = xmax.max(c[0]);
        ymax = ymax.max(c[1]);
        zmax = zmax.max(c[2]);
    }
    if !xmin.is_finite() {
        xmin = 0.0;
        ymin = 0.0;
        zmin = 0.0;
        xmax = 1.0;
        ymax = 1.0;
        zmax = 1.0;
    }
    let body = json!({
        "positions": mesh.positions,
        "indices": mesh.indices,
        "vertex_count": mesh.vertex_count(),
        "triangle_count": mesh.triangle_count(),
        "bbox_mm": {
            "min": [xmin, ymin, zmin],
            "max": [xmax, ymax, zmax],
        },
        "note": "coarse preview mesh for canvas 3D (H2-6); not STEP/GLB parity",
    });
    fs::write(
        path,
        serde_json::to_string(&body).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

/// H3-5: place `drawing.json` in snap dir — prefer sibling `*.drawing.json`, else auto-dims.
fn write_drawing_for_view(
    source: &Path,
    snap: &Path,
    ir: &cadrion_lang::FeatureIr,
) -> Result<(), String> {
    let dest = snap.join("drawing.json");
    // sibling next to .cad.star: stem.drawing.json or name.drawing.json
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("part");
    let stem = stem.strip_suffix(".cad").unwrap_or(stem);
    let parent = source.parent().unwrap_or_else(|| Path::new("."));
    let candidates = [
        parent.join(format!("{stem}.drawing.json")),
        source.with_extension("drawing.json"),
    ];
    for c in &candidates {
        if c.is_file() {
            fs::copy(c, &dest).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    // auto opposite-face dims
    let topo = crate::topo_from_ir::topology_from_ir(ir)?;
    let name = source
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("part");
    let packet = cadrion_inspect::build_drawing_packet(&topo, name, "ir-analytic-view", &[]);
    fs::write(
        &dest,
        serde_json::to_string_pretty(&packet).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn handle_client(stream: &mut TcpStream, entries: &[Entry], base: &str) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let line = req.lines().next().unwrap_or("");
    let path = line.split_whitespace().nth(1).unwrap_or("/");

    if path == "/" || path == "/index.html" {
        let html = index_html(entries, base);
        return respond(stream, "text/html; charset=utf-8", html.as_bytes());
    }

    if let Some(rest) = path.strip_prefix("/v/") {
        let mut parts = rest.splitn(2, '/');
        let idx: usize = parts.next().unwrap_or("0").parse().unwrap_or(9999);
        let file = parts.next().unwrap_or("");
        if idx >= entries.len() {
            return respond(stream, "text/plain", b"not found");
        }
        let ent = &entries[idx];
        if file.is_empty() {
            let html = match ent.kind {
                "gcode" => gcode_html(ent, idx, base),
                "robot" => robot_html(ent, idx, base),
                "star" | "snap" => mesh_packet_html(ent, idx, base),
                _ => packet_html(ent, idx, base),
            };
            return respond(stream, "text/html; charset=utf-8", html.as_bytes());
        }
        let file = file.trim_start_matches('/');
        if file.contains("..") || file.contains('/') || file.contains('\\') {
            return respond(stream, "text/plain", b"bad path");
        }
        let fp = ent.root.join(file);
        if fp.is_file() {
            let data = fs::read(&fp)?;
            return respond(stream, content_type(&fp), &data);
        }
    }

    respond(stream, "text/plain", b"not found")
}

fn content_type(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("json") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn respond(stream: &mut TcpStream, ct: &str, body: &[u8]) -> std::io::Result<()> {
    let hdr = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(hdr.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn index_html(entries: &[Entry], base: &str) -> String {
    let mut items = String::new();
    for (i, e) in entries.iter().enumerate() {
        items.push_str(&format!(
            "<li><a href=\"{base}/v/{i}/\">{path}</a> <small>{kind}</small></li>",
            base = base,
            i = i,
            path = e.path.display(),
            kind = e.kind
        ));
    }
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Cadrion View</title>
<style>
body{{font-family:system-ui,sans-serif;background:#12141a;color:#e8eaed;margin:2rem}}
a{{color:#8ab4f8}} li{{margin:.5rem 0}}
</style></head><body>
<h1>Cadrion Viewer <small style="opacity:.6">alpha</small></h1>
<p style="opacity:.7">snaps · mesh 3D · gcode scrub · urdf jog 3D</p>
<ul>{items}</ul>
</body></html>"#
    )
}

fn packet_html(ent: &Entry, idx: usize, base: &str) -> String {
    let mut imgs = String::new();
    if let Ok(rd) = fs::read_dir(&ent.root) {
        let mut files: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        files.sort_by_key(|e| e.file_name());
        for f in files {
            let name = f.file_name().to_string_lossy().into_owned();
            if name.ends_with(".png") || name.ends_with(".gif") {
                imgs.push_str(&format!(
                    "<figure><img src=\"{base}/v/{idx}/{name}\" alt=\"{name}\"><figcaption>{name}</figcaption></figure>",
                    base = base,
                    idx = idx,
                    name = name
                ));
            }
        }
    }
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>{title}</title>
<style>
body{{font-family:system-ui,sans-serif;background:#12141a;color:#e8eaed;margin:1.5rem}}
a{{color:#8ab4f8}}
.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:1rem}}
img{{max-width:100%;background:#1a1d24;border-radius:8px}}
figcaption{{opacity:.7;font-size:.85rem;margin-top:.35rem}}
</style></head><body>
<p><a href="{base}/">← all</a></p>
<h1>{title}</h1>
<p style="opacity:.7">{kind} · {root}</p>
<div class="grid">{imgs}</div>
</body></html>"#,
        title = ent.path.display(),
        base = base,
        kind = ent.kind,
        root = ent.root.display(),
        imgs = imgs
    )
}

/// Snap / star packet with optional interactive mesh 3D (H2-6).
fn mesh_packet_html(ent: &Entry, idx: usize, base: &str) -> String {
    let mut imgs = String::new();
    if let Ok(rd) = fs::read_dir(&ent.root) {
        let mut files: Vec<_> = rd.filter_map(|e| e.ok()).collect();
        files.sort_by_key(|e| e.file_name());
        for f in files {
            let name = f.file_name().to_string_lossy().into_owned();
            if name.ends_with(".png") || name.ends_with(".gif") {
                imgs.push_str(&format!(
                    "<figure><img src=\"{base}/v/{idx}/{name}\" alt=\"{name}\"><figcaption>{name}</figcaption></figure>",
                    base = base,
                    idx = idx,
                    name = name
                ));
            }
        }
    }
    let mesh_link = if ent.root.join("mesh.json").is_file() {
        format!("{base}/v/{idx}/mesh.json")
    } else {
        String::new()
    };
    let drawing_link = if ent.root.join("drawing.json").is_file() {
        format!("{base}/v/{idx}/drawing.json")
    } else {
        String::new()
    };
    let mesh_block = if mesh_link.is_empty() {
        String::new()
    } else {
        format!(
            r#"
<h2>Interactive mesh <small style="opacity:.6">H2-6 · drag to orbit · H3-5 PMI labels</small></h2>
<p class="meta"><a href="{mesh}">mesh.json</a>{draw_a} · coarse preview, not GLB/STEP · not a drafting package</p>
<canvas id="m3" width="720" height="480"></canvas>
<ul id="dimlist" class="dims"></ul>
<script>
const meshUrl = "{mesh}";
const drawingUrl = "{drawing}";
const cv = document.getElementById('m3');
const cx = cv.getContext('2d');
let mesh=null, drawing=null, yaw=0.7, pitch=0.45, drag=false, lx=0, ly=0;
cv.addEventListener('pointerdown', e => {{ drag=true; lx=e.clientX; ly=e.clientY; cv.setPointerCapture(e.pointerId); }});
cv.addEventListener('pointerup', () => drag=false);
cv.addEventListener('pointermove', e => {{
  if (!drag) return;
  yaw += (e.clientX-lx)*0.01; pitch += (e.clientY-ly)*0.01;
  pitch = Math.max(-1.4, Math.min(1.4, pitch));
  lx=e.clientX; ly=e.clientY; draw();
}});
function rot(p, yaw, pitch) {{
  let [x,y,z]=p;
  let c=Math.cos(yaw), s=Math.sin(yaw);
  let x1=x*c - z*s, z1=x*s + z*c;
  c=Math.cos(pitch); s=Math.sin(pitch);
  let y2=y*c - z1*s, z2=y*s + z1*c;
  return [x1,y2,z2];
}}
function draw() {{
  if (!mesh) return;
  cx.clearRect(0,0,cv.width,cv.height);
  const pos=mesh.positions, idx=mesh.indices;
  const bb=mesh.bbox_mm||{{min:[0,0,0],max:[1,1,1]}};
  const cx0=(bb.min[0]+bb.max[0])/2, cy0=(bb.min[1]+bb.max[1])/2, cz0=(bb.min[2]+bb.max[2])/2;
  const dx=bb.max[0]-bb.min[0], dy=bb.max[1]-bb.min[1], dz=bb.max[2]-bb.min[2];
  const sc=Math.min(cv.width,cv.height)*0.7/Math.max(1e-6, Math.max(dx,dy,dz));
  const tris=[];
  for (let i=0;i<idx.length;i+=3) {{
    const ia=idx[i]*3, ib=idx[i+1]*3, ic=idx[i+2]*3;
    const A=rot([pos[ia]-cx0,pos[ia+1]-cy0,pos[ia+2]-cz0], yaw, pitch);
    const B=rot([pos[ib]-cx0,pos[ib+1]-cy0,pos[ib+2]-cz0], yaw, pitch);
    const C=rot([pos[ic]-cx0,pos[ic+1]-cy0,pos[ic+2]-cz0], yaw, pitch);
    const z=(A[2]+B[2]+C[2])/3;
    const e1=[B[0]-A[0],B[1]-A[1],B[2]-A[2]], e2=[C[0]-A[0],C[1]-A[1],C[2]-A[2]];
    const nx=e1[1]*e2[2]-e1[2]*e2[1], ny=e1[2]*e2[0]-e1[0]*e2[2], nz=e1[0]*e2[1]-e1[1]*e2[0];
    if (nz<=0) continue;
    const shade=0.35+0.65*Math.min(1, Math.max(0, nz/Math.hypot(nx,ny,nz)));
    tris.push({{A,B,C,z,shade}});
  }}
  tris.sort((a,b)=>a.z-b.z);
  const ox=cv.width/2, oy=cv.height/2;
  for (const t of tris) {{
    const g=Math.floor(80+140*t.shade);
    cx.fillStyle=`rgb(${{g}},${{g+10}},${{g+25}})`;
    cx.beginPath();
    cx.moveTo(ox+t.A[0]*sc, oy-t.A[1]*sc);
    cx.lineTo(ox+t.B[0]*sc, oy-t.B[1]*sc);
    cx.lineTo(ox+t.C[0]*sc, oy-t.C[1]*sc);
    cx.closePath(); cx.fill();
  }}
  // H3-5 PMI HUD: dim value chips (not true leader lines / drafting)
  if (drawing && drawing.dims && drawing.dims.length) {{
    cx.font = '13px system-ui,sans-serif';
    cx.textBaseline = 'top';
    let y = 10;
    for (const d of drawing.dims) {{
      const label = (d.label ? d.label + ': ' : '') + d.value.toFixed(2) + ' ' + (d.unit||'mm');
      const tw = cx.measureText(label).width;
      cx.fillStyle = 'rgba(18,20,26,0.82)';
      cx.fillRect(10, y, tw + 16, 22);
      cx.strokeStyle = '#fdd663';
      cx.strokeRect(10.5, y+0.5, tw + 15, 21);
      cx.fillStyle = '#fdd663';
      cx.fillText(label, 18, y + 4);
      y += 26;
    }}
    cx.fillStyle = 'rgba(232,234,237,0.55)';
    cx.font = '11px system-ui,sans-serif';
    cx.fillText('PMI alpha · not a drafting package', 10, cv.height - 18);
  }}
}}
function fillDimList() {{
  const ul = document.getElementById('dimlist');
  if (!ul || !drawing || !drawing.dims) return;
  ul.innerHTML = '';
  for (const d of drawing.dims) {{
    const li = document.createElement('li');
    li.textContent = (d.id||'') + ' · ' + (d.kind||'linear') + ' · ' +
      d.value.toFixed(3) + ' ' + (d.unit||'mm') +
      (d.label ? ' (' + d.label + ')' : '') +
      (d.a ? '  ' + d.a + (d.b ? ' ↔ ' + d.b : '') : '');
    ul.appendChild(li);
  }}
}}
(async()=>{{
  mesh = await (await fetch(meshUrl)).json();
  if (drawingUrl) {{
    try {{ drawing = await (await fetch(drawingUrl)).json(); fillDimList(); }} catch (_) {{}}
  }}
  draw();
}})().catch(e=>document.body.insertAdjacentHTML('beforeend','<pre>'+e+'</pre>'));
</script>
"#,
            mesh = mesh_link,
            drawing = drawing_link,
            draw_a = if drawing_link.is_empty() {
                String::new()
            } else {
                format!(r#" · <a href="{drawing_link}">drawing.json</a>"#)
            },
        )
    };
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>{title}</title>
<style>
body{{font-family:system-ui,sans-serif;background:#12141a;color:#e8eaed;margin:1.5rem}}
a{{color:#8ab4f8}}
.grid{{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:1rem}}
img{{max-width:100%;background:#1a1d24;border-radius:8px}}
figcaption{{opacity:.7;font-size:.85rem;margin-top:.35rem}}
#m3{{background:#1a1d24;border-radius:8px;width:min(720px,100%);height:480px;display:block;touch-action:none;cursor:grab}}
.meta{{opacity:.75;font-size:.9rem}}
.dims{{list-style:none;padding:0;margin:1rem 0;max-width:720px}}
.dims li{{background:#1a1d24;border-radius:6px;padding:.45rem .7rem;margin:.35rem 0;font-size:.9rem;border-left:3px solid #fdd663}}
</style></head><body>
<p><a href="{base}/">← all</a></p>
<h1>{title}</h1>
<p style="opacity:.7">{kind} · {root}</p>
{mesh_block}
<h2>Snapshot views</h2>
<div class="grid">{imgs}</div>
</body></html>"#,
        title = ent.path.display(),
        base = base,
        kind = ent.kind,
        root = ent.root.display(),
        imgs = imgs,
        mesh_block = mesh_block
    )
}

fn gcode_html(ent: &Entry, idx: usize, base: &str) -> String {
    let meta = ent.meta_name.as_deref().unwrap_or("path.json");
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>G-code scrub · {title}</title>
<style>
body{{font-family:system-ui,sans-serif;background:#12141a;color:#e8eaed;margin:1.5rem}}
a{{color:#8ab4f8}}
canvas{{background:#1a1d24;border-radius:8px;width:min(720px,100%);height:420px;display:block;touch-action:none}}
.row{{display:flex;gap:1rem;flex-wrap:wrap;align-items:center;margin:1rem 0}}
input[type=range]{{width:min(420px,70vw)}}
.meta{{opacity:.75;font-size:.9rem}}
</style></head><body>
<p><a href="{base}/">← all</a></p>
<h1>G-code layer scrub + 3D path</h1>
<p class="meta">{title} · <a href="{base}/v/{idx}/{meta}">{meta}</a></p>
<h2>Layer (XY)</h2>
<canvas id="c" width="720" height="420"></canvas>
<div class="row">
  <label>layer <span id="li">0</span> / <span id="ln">0</span> · Z=<span id="lz">0</span></label>
  <input id="slider" type="range" min="0" max="0" value="0">
</div>
<h2>3D path <small style="opacity:.6">drag to orbit · layer filter</small></h2>
<canvas id="c3" width="720" height="420"></canvas>
<p class="meta">Blue = extrude · grey = travel · not a physics sim</p>
<script>
const metaUrl = "{base}/v/{idx}/{meta}";
const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
const c3 = document.getElementById('c3');
const x3 = c3.getContext('2d');
let data = null, layerI=0, yaw=0.8, pitch=0.5, drag=false, lx=0, ly=0;
c3.addEventListener('pointerdown', e => {{ drag=true; lx=e.clientX; ly=e.clientY; c3.setPointerCapture(e.pointerId); }});
c3.addEventListener('pointerup', () => drag=false);
c3.addEventListener('pointermove', e => {{
  if (!drag) return;
  yaw += (e.clientX-lx)*0.01; pitch += (e.clientY-ly)*0.01;
  pitch = Math.max(-1.3, Math.min(1.3, pitch));
  lx=e.clientX; ly=e.clientY; draw3();
}});
async function main() {{
  data = await (await fetch(metaUrl)).json();
  const n = Math.max(0, (data.layers||[]).length - 1);
  document.getElementById('ln').textContent = n;
  const sl = document.getElementById('slider');
  sl.max = n;
  sl.oninput = () => {{ layerI=+sl.value; draw2(); draw3(); }};
  draw2(); draw3();
}}
function draw2() {{
  const li=layerI; const L = data.layers[li];
  if (!L) return;
  document.getElementById('li').textContent = li;
  document.getElementById('lz').textContent = L.z.toFixed(3);
  const pts = data.points.slice(L.start, L.end);
  ctx.clearRect(0,0,canvas.width,canvas.height);
  if (pts.length < 2) return;
  let xmin=Infinity,xmax=-Infinity,ymin=Infinity,ymax=-Infinity;
  for (const p of pts) {{ xmin=Math.min(xmin,p.x); xmax=Math.max(xmax,p.x); ymin=Math.min(ymin,p.y); ymax=Math.max(ymax,p.y); }}
  const pad=24;
  const sx = (canvas.width-2*pad)/Math.max(1e-6, xmax-xmin);
  const sy = (canvas.height-2*pad)/Math.max(1e-6, ymax-ymin);
  const s = Math.min(sx,sy);
  const ox = pad - xmin*s;
  const oy = canvas.height - pad + ymin*s;
  const map = p => [ox + p.x*s, oy - p.y*s];
  ctx.lineWidth = 1.5;
  for (let i=1;i<pts.length;i++) {{
    const a=pts[i-1], b=pts[i];
    ctx.beginPath();
    const [x0,y0]=map(a), [x1,y1]=map(b);
    ctx.moveTo(x0,y0); ctx.lineTo(x1,y1);
    ctx.strokeStyle = b.extrude ? '#8ab4f8' : '#5f6368';
    ctx.stroke();
  }}
}}
function rot(p) {{
  let [x,y,z]=p;
  let c=Math.cos(yaw), s=Math.sin(yaw);
  let x1=x*c-z*s, z1=x*s+z*c;
  c=Math.cos(pitch); s=Math.sin(pitch);
  return [x1, y*c-z1*s, y*s+z1*c];
}}
function draw3() {{
  if (!data) return;
  const L = data.layers[layerI];
  const pts = L ? data.points.slice(0, L.end) : data.points;
  x3.clearRect(0,0,c3.width,c3.height);
  if (!pts || pts.length<2) return;
  let xmin=Infinity,xmax=-Infinity,ymin=Infinity,ymax=-Infinity,zmin=Infinity,zmax=-Infinity;
  for (const p of pts) {{
    xmin=Math.min(xmin,p.x); xmax=Math.max(xmax,p.x);
    ymin=Math.min(ymin,p.y); ymax=Math.max(ymax,p.y);
    zmin=Math.min(zmin,p.z); zmax=Math.max(zmax,p.z);
  }}
  const cx0=(xmin+xmax)/2, cy0=(ymin+ymax)/2, cz0=(zmin+zmax)/2;
  const span=Math.max(xmax-xmin, ymax-ymin, zmax-zmin, 1e-6);
  const sc=Math.min(c3.width,c3.height)*0.7/span;
  const ox=c3.width/2, oy=c3.height/2;
  x3.lineWidth=1.4; x3.lineCap='round';
  for (let i=1;i<pts.length;i++) {{
    const a=pts[i-1], b=pts[i];
    const A=rot([a.x-cx0,a.y-cy0,a.z-cz0]);
    const B=rot([b.x-cx0,b.y-cy0,b.z-cz0]);
    x3.beginPath();
    x3.moveTo(ox+A[0]*sc, oy-A[1]*sc);
    x3.lineTo(ox+B[0]*sc, oy-B[1]*sc);
    x3.strokeStyle = b.extrude ? '#8ab4f8' : '#5f6368';
    x3.stroke();
  }}
}}
main().catch(e => {{ document.body.insertAdjacentHTML('beforeend','<pre>'+e+'</pre>'); }});
</script>
</body></html>"#,
        title = ent.path.display(),
        base = base,
        idx = idx,
        meta = meta,
    )
}

fn robot_html(ent: &Entry, idx: usize, base: &str) -> String {
    let meta = ent.meta_name.as_deref().unwrap_or("robot.json");
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>URDF jog · {title}</title>
<style>
body{{font-family:system-ui,sans-serif;background:#12141a;color:#e8eaed;margin:1.5rem}}
a{{color:#8ab4f8}}
#c{{background:#1a1d24;border-radius:8px;width:min(720px,100%);height:480px;display:block;touch-action:none;cursor:grab}}
.sliders{{display:grid;gap:.75rem;max-width:720px;margin-top:1rem}}
label{{display:flex;flex-direction:column;gap:.25rem;font-size:.9rem}}
input[type=range]{{width:100%}}
.meta{{opacity:.75;font-size:.9rem}}
</style></head><body>
<p><a href="{base}/">← all</a></p>
<h1>URDF joint jog <small style="opacity:.6">H2-6 · 3D stick FK</small></h1>
<p class="meta">{title} · <a href="{base}/v/{idx}/{meta}">{meta}</a> · drag canvas to orbit</p>
<canvas id="c" width="720" height="480"></canvas>
<div class="sliders" id="sliders"></div>
<p class="meta">Stick figure FK with joint limits — not mesh/GLB. Not Blender.</p>
<script>
const metaUrl = "{base}/v/{idx}/{meta}";
const canvas = document.getElementById('c');
const ctx = canvas.getContext('2d');
let robot = null;
const q = {{}};
let yaw=0.9, pitch=0.4, drag=false, lx=0, ly=0;
canvas.addEventListener('pointerdown', e => {{ drag=true; lx=e.clientX; ly=e.clientY; canvas.setPointerCapture(e.pointerId); }});
canvas.addEventListener('pointerup', () => drag=false);
canvas.addEventListener('pointermove', e => {{
  if (!drag) return;
  yaw += (e.clientX-lx)*0.01; pitch += (e.clientY-ly)*0.01;
  pitch = Math.max(-1.3, Math.min(1.3, pitch));
  lx=e.clientX; ly=e.clientY; draw();
}});
async function main() {{
  robot = await (await fetch(metaUrl)).json();
  const box = document.getElementById('sliders');
  for (const j of robot.joints) {{
    if (!j.movable) continue;
    q[j.name] = 0;
    const lab = document.createElement('label');
    lab.innerHTML = j.name+' ('+j.joint_type+') <span id="v_'+j.name+'">0</span>';
    const inp = document.createElement('input');
    inp.type = 'range';
    inp.min = j.lower; inp.max = j.upper; inp.step = (j.upper-j.lower)/200 || 0.01;
    inp.value = 0;
    inp.oninput = () => {{ q[j.name]=+inp.value; document.getElementById('v_'+j.name).textContent=inp.value; draw(); }};
    lab.appendChild(inp); box.appendChild(lab);
  }}
  draw();
}}
function matMul(A,B) {{
  const C=new Array(16).fill(0);
  for (let r=0;r<4;r++) for (let c=0;c<4;c++) for (let k=0;k<4;k++) C[r*4+c]+=A[r*4+k]*B[k*4+c];
  return C;
}}
function matT(x,y,z) {{ return [1,0,0,x, 0,1,0,y, 0,0,1,z, 0,0,0,1]; }}
function matRx(a) {{ const c=Math.cos(a),s=Math.sin(a); return [1,0,0,0, 0,c,-s,0, 0,s,c,0, 0,0,0,1]; }}
function matRy(a) {{ const c=Math.cos(a),s=Math.sin(a); return [c,0,s,0, 0,1,0,0, -s,0,c,0, 0,0,0,1]; }}
function matRz(a) {{ const c=Math.cos(a),s=Math.sin(a); return [c,-s,0,0, s,c,0,0, 0,0,1,0, 0,0,0,1]; }}
function matAxis(ax, ay, az, ang) {{
  const n=Math.hypot(ax,ay,az)||1; ax/=n; ay/=n; az/=n;
  const c=Math.cos(ang), s=Math.sin(ang), t=1-c;
  return [
    t*ax*ax+c, t*ax*ay-s*az, t*ax*az+s*ay, 0,
    t*ax*ay+s*az, t*ay*ay+c, t*ay*az-s*ax, 0,
    t*ax*az-s*ay, t*ay*az+s*ax, t*az*az+c, 0,
    0,0,0,1
  ];
}}
function originMat(j) {{
  let M = matT(j.origin_xyz[0], j.origin_xyz[1], j.origin_xyz[2]);
  M = matMul(M, matRz(j.origin_rpy[2]||0));
  M = matMul(M, matRy(j.origin_rpy[1]||0));
  M = matMul(M, matRx(j.origin_rpy[0]||0));
  return M;
}}
function apply(M,x,y,z) {{
  return [M[0]*x+M[1]*y+M[2]*z+M[3], M[4]*x+M[5]*y+M[6]*z+M[7], M[8]*x+M[9]*y+M[10]*z+M[11]];
}}
function project(p) {{
  let [x,y,z]=p;
  let c=Math.cos(yaw), s=Math.sin(yaw);
  let x1=x*c-z*s, z1=x*s+z*c;
  c=Math.cos(pitch); s=Math.sin(pitch);
  let y2=y*c-z1*s, z2=y*s+z1*c;
  const sc=280, ox=canvas.width/2, oy=canvas.height*0.65;
  return [ox+x1*sc, oy-y2*sc, z2];
}}
function draw() {{
  ctx.clearRect(0,0,canvas.width,canvas.height);
  const parentJ = {{}};
  for (const j of robot.joints) parentJ[j.child] = j;
  const poses = {{}};
  poses[robot.root] = [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1];
  for (let k=0;k<robot.joints.length+3;k++) {{
    for (const j of robot.joints) {{
      if (!poses[j.parent] || poses[j.child]) continue;
      let M = matMul(poses[j.parent], originMat(j));
      const qq = q[j.name]||0;
      if (j.joint_type==='revolute' || j.joint_type==='continuous') {{
        M = matMul(M, matAxis(j.axis[0], j.axis[1], j.axis[2], qq));
      }} else if (j.joint_type==='prismatic') {{
        M = matMul(M, matT(j.axis[0]*qq, j.axis[1]*qq, j.axis[2]*qq));
      }}
      poses[j.child] = M;
    }}
  }}
  // ground
  ctx.strokeStyle='#3c4043'; ctx.beginPath();
  const g0=project([-0.3,0,-0.3]), g1=project([0.3,0,-0.3]), g2=project([0.3,0,0.3]), g3=project([-0.3,0,0.3]);
  ctx.moveTo(g0[0],g0[1]); ctx.lineTo(g1[0],g1[1]); ctx.lineTo(g2[0],g2[1]); ctx.lineTo(g3[0],g3[1]); ctx.closePath(); ctx.stroke();
  const segs=[];
  ctx.lineWidth=5; ctx.lineCap='round';
  for (const j of robot.joints) {{
    const Mp=poses[j.parent], Mc=poses[j.child];
    if (!Mp||!Mc) continue;
    const a=apply(Mp,0,0,0), b=apply(Mc,0,0,0);
    const pa=project(a), pb=project(b);
    segs.push({{pa,pb,z:(pa[2]+pb[2])/2, mov:j.movable}});
  }}
  segs.sort((u,v)=>u.z-v.z);
  for (const s of segs) {{
    ctx.strokeStyle = s.mov ? '#8ab4f8' : '#9aa0a6';
    ctx.beginPath(); ctx.moveTo(s.pa[0],s.pa[1]); ctx.lineTo(s.pb[0],s.pb[1]); ctx.stroke();
    ctx.fillStyle='#fdd663';
    ctx.beginPath(); ctx.arc(s.pb[0],s.pb[1],5,0,Math.PI*2); ctx.fill();
  }}
  const Mr=poses[robot.root];
  if (Mr) {{
    const p=project(apply(Mr,0,0,0));
    ctx.fillStyle='#81c995';
    ctx.fillRect(p[0]-8,p[1]-8,16,16);
  }}
}}
main().catch(e => {{ document.body.insertAdjacentHTML('beforeend','<pre>'+e+'</pre>'); }});
</script>
</body></html>"#,
        title = ent.path.display(),
        base = base,
        idx = idx,
        meta = meta,
    )
}
