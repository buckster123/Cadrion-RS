//! Run harness tasks (scripted + live).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cadrion_inspect::{inspect_refs, TopologySnapshot};
use cadrion_kernel::{GeomKernel, MockKernel, ShapeFacts};
use cadrion_lang::{evaluate, execute_ir, EvalOptions, FeatureIr};
use cadrion_render::{mesh_from_ir, write_snapshot_packet, SnapshotOptions, ViewName};

use crate::live::{run_task_live, LiveOpts};
use crate::scenario::{AssertSpec, Step, Task};
use crate::score::{Scorecard, TaskResult, SUITE_AGENT10};

#[derive(Debug, Clone, Default)]
pub struct RunOpts {
    /// Override tasks directory.
    pub tasks_root: Option<PathBuf>,
    /// When set, run live external agent instead of scripted loops.
    pub live: Option<LiveOpts>,
}

/// Mutable state for one scripted/live verify loop.
#[derive(Debug)]
pub(crate) struct LoopState {
    pub(crate) last_facts: Option<ShapeFacts>,
    pub(crate) last_ir: Option<FeatureIr>,
    pub(crate) last_label: Option<String>,
    pub(crate) last_snap: Option<TopologySnapshot>,
    pub(crate) snapshot_ok: bool,
    pub(crate) work: PathBuf,
}

/// Default `harness/tasks` next to repo root.
pub fn default_tasks_root() -> PathBuf {
    let from_crate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/tasks");
    if from_crate.is_dir() {
        return from_crate.canonicalize().unwrap_or(from_crate);
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cand = cwd.join("harness/tasks");
    if cand.is_dir() {
        return cand;
    }
    cwd.join("harness/tasks")
}

pub fn run_suite(suite: &str, opts: &RunOpts) -> Result<Scorecard, String> {
    let started = Instant::now();
    let root = opts.tasks_root.clone().unwrap_or_else(default_tasks_root);
    if !root.is_dir() {
        return Err(format!("tasks root missing: {}", root.display()));
    }
    let ids = match suite {
        SUITE_AGENT10 | "agent" | "m3-harness" => {
            (1..=10).map(|i| format!("{i:02}")).collect::<Vec<_>>()
        }
        other => return Err(format!("unknown harness suite: {other}")),
    };
    let mode = if opts.live.is_some() {
        "live"
    } else {
        "scripted"
    };
    let mut results = Vec::new();
    for id_prefix in ids {
        let path = find_task_file(&root, &id_prefix)?;
        let task = load_task(&path)?;
        let r = if let Some(live) = &opts.live {
            run_task_live(&task, &path, live)?
        } else {
            run_task(&task, &root)?
        };
        results.push(r);
    }
    let mut card =
        Scorecard::from_tasks(suite, mode, results, started.elapsed().as_millis() as u64);
    if let Some(live) = &opts.live {
        let model = live_model_id(&live.cmd);
        card = card.with_provenance(live.cmd.clone(), model);
        if live.cmd.trim() == "@oracle" || live.cmd.contains("oracle_agent") {
            card = card
                .with_note("oracle cheats via task file — plumbing/control only, not an LLM score");
        }
        if let Ok(note) = std::env::var("CADRION_HARNESS_NOTES") {
            let note = note.trim();
            if !note.is_empty() {
                card = card.with_note(note);
            }
        }
    } else {
        card = card.with_provenance("", "scripted-builtin");
    }
    Ok(card)
}

/// Live `model_id`: oracle markers, else `$CADRION_HARNESS_MODEL_ID`, else `external-cmd`.
fn live_model_id(cmd: &str) -> String {
    if cmd.trim() == "@oracle" {
        return "oracle:in-process".into();
    }
    if cmd.contains("oracle_agent") {
        return "oracle:python".into();
    }
    std::env::var("CADRION_HARNESS_MODEL_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "external-cmd".into())
}

fn find_task_file(root: &Path, id_prefix: &str) -> Result<PathBuf, String> {
    let rd = fs::read_dir(root).map_err(|e| e.to_string())?;
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with(id_prefix) {
            return Ok(p);
        }
    }
    Err(format!(
        "no task json starting with '{id_prefix}' under {}",
        root.display()
    ))
}

fn load_task(path: &Path) -> Result<Task, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

pub fn run_task(task: &Task, tasks_root: &Path) -> Result<TaskResult, String> {
    let started = Instant::now();
    let work = std::env::temp_dir().join(format!(
        "cadrion-harness-{}-{}",
        task.id,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;

    let mut last_err = String::from("no loops defined");
    let max = task.max_loops.min(task.loops.len() as u32).max(1);

    for (i, steps) in task.loops.iter().enumerate() {
        let loop_n = (i as u32) + 1;
        if loop_n > task.max_loops {
            break;
        }
        let mut st = LoopState {
            last_facts: None,
            last_ir: None,
            last_label: None,
            last_snap: None,
            snapshot_ok: false,
            work: work.clone(),
        };
        match run_loop(steps, &mut st) {
            Ok(()) => {
                let _ = fs::remove_dir_all(&work);
                return Ok(TaskResult {
                    id: task.id.clone(),
                    ok: true,
                    loops_used: loop_n,
                    max_loops: task.max_loops,
                    wall_ms: started.elapsed().as_millis() as u64,
                    detail: format!("passed on loop {loop_n}/{max}"),
                    prompt: task.prompt.clone(),
                    mode: "scripted".into(),
                });
            }
            Err(e) => {
                last_err = format!("loop {loop_n}: {e}");
            }
        }
    }

    let _ = fs::remove_dir_all(&work);
    let _ = tasks_root; // reserved for future relative assets
    Ok(TaskResult {
        id: task.id.clone(),
        ok: false,
        loops_used: max,
        max_loops: task.max_loops,
        wall_ms: started.elapsed().as_millis() as u64,
        detail: last_err,
        prompt: task.prompt.clone(),
        mode: "scripted".into(),
    })
}

fn run_loop(steps: &[Step], st: &mut LoopState) -> Result<(), String> {
    for step in steps {
        match step {
            Step::Write { path, content } => {
                let p = st.work.join(path);
                if let Some(parent) = p.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::write(&p, content).map_err(|e| format!("write {}: {e}", p.display()))?;
            }
            Step::Build { path } => {
                let p = st.work.join(path);
                let src =
                    fs::read_to_string(&p).map_err(|e| format!("read {}: {e}", p.display()))?;
                let name = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("part.cad.star");
                let eval = evaluate(&src, &EvalOptions::new(name));
                if !eval.ok {
                    return Err(format!("eval failed: {:?}", eval.diagnostics));
                }
                let ir = eval.ir.ok_or("missing ir")?;
                st.last_label = ir.label.clone();
                let mut k = MockKernel::new();
                let sid = execute_ir(&mut k, &ir).map_err(|e| e.to_string())?;
                let facts = k.facts(sid).map_err(|e| e.to_string())?;
                // topology from IR for inspect (reuse bench-style walker via inspect helpers)
                let snap = topo_from_ir(&ir)?;
                st.last_facts = Some(facts);
                st.last_ir = Some(ir);
                st.last_snap = Some(snap);
            }
            Step::InspectRefs { path, facts } => {
                let _ = path;
                let snap = st.last_snap.as_ref().ok_or("inspect_refs: build first")?;
                let report = inspect_refs(snap, *facts);
                if report.faces == 0 {
                    return Err("inspect_refs: zero faces".into());
                }
                // stash nothing extra; asserts read report via rebuild
                st.last_snap = Some(snap.clone());
            }
            Step::Snapshot { path, size } => {
                let p = st.work.join(path);
                let ir = st.last_ir.as_ref().ok_or("snapshot: build first")?;
                let (mesh, _notes) = mesh_from_ir(ir).map_err(|e| format!("mesh: {e}"))?;
                let out = st.work.join("snap_out");
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                let opts = SnapshotOptions {
                    views: vec![ViewName::Iso, ViewName::Front],
                    width: *size,
                    height: *size,
                    gif: true,
                    gif_frames: 8,
                    gif_delay_cs: 6,
                    notes: vec!["harness".into()],
                };
                let r = write_snapshot_packet(&mesh, &out, &opts)
                    .map_err(|e| format!("snapshot: {e}"))?;
                if r.manifest.views.is_empty() {
                    return Err("snapshot produced no views".into());
                }
                let _ = p;
                st.snapshot_ok = true;
            }
            Step::Assert {
                volume_min,
                volume_max,
                faces_min,
                label,
                has_selector_prefix,
                snapshot_ok,
            } => {
                apply_assert(
                    &AssertSpec {
                        volume_min: *volume_min,
                        volume_max: *volume_max,
                        faces_min: *faces_min,
                        label: label.clone(),
                        has_selector_prefix: has_selector_prefix.clone(),
                        snapshot_ok: *snapshot_ok,
                    },
                    st,
                )?;
            }
        }
    }
    Ok(())
}

fn apply_assert(a: &AssertSpec, st: &LoopState) -> Result<(), String> {
    if let Some(facts) = &st.last_facts {
        if let Some(vmin) = a.volume_min {
            if facts.volume_mm3 + 1e-6 < vmin {
                return Err(format!("volume {} < min {vmin}", facts.volume_mm3));
            }
        }
        if let Some(vmax) = a.volume_max {
            if facts.volume_mm3 - 1e-6 > vmax {
                return Err(format!("volume {} > max {vmax}", facts.volume_mm3));
            }
        }
    } else if a.volume_min.is_some() || a.volume_max.is_some() {
        return Err("assert volume: no facts (build first)".into());
    }

    if let Some(n) = a.faces_min {
        let snap = st.last_snap.as_ref().ok_or("assert faces: no topology")?;
        let faces = snap
            .solids
            .iter()
            .map(|s| s.faces.len() as u32)
            .sum::<u32>();
        if faces < n {
            return Err(format!("faces {faces} < min {n}"));
        }
    }

    if let Some(want) = &a.label {
        match &st.last_label {
            Some(got) if got == want => {}
            Some(got) => return Err(format!("label got {got:?} want {want}")),
            None => return Err(format!("label missing, want {want}")),
        }
    }

    if let Some(prefix) = &a.has_selector_prefix {
        let snap = st
            .last_snap
            .as_ref()
            .ok_or("assert selector: no topology")?;
        let report = inspect_refs(snap, false);
        if !report.refs.iter().any(|r| r.selector.starts_with(prefix)) {
            return Err(format!("no selector starting with {prefix}"));
        }
    }

    if a.snapshot_ok && !st.snapshot_ok {
        return Err("snapshot_ok required but snapshot not run/failed".into());
    }
    Ok(())
}

pub(crate) fn topo_from_ir(ir: &FeatureIr) -> Result<TopologySnapshot, String> {
    use cadrion_inspect::{box_topology, cylinder_topology, SolidRec};
    use cadrion_kernel::Point3;
    use cadrion_lang::{BooleanKind, IrNode};

    let mut solids: Vec<Option<SolidRec>> = vec![None; ir.nodes.len()];
    for (idx, node) in ir.nodes.iter().enumerate() {
        let rec = match node {
            IrNode::Box { dx, dy, dz, at } => {
                box_topology(*dx, *dy, *dz, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Cylinder { radius, height, at } => {
                cylinder_topology(*radius, *height, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Sphere { radius, at } => {
                let r = *radius;
                box_topology(2.0 * r, 2.0 * r, 2.0 * r, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Cone { radius, height, at } => {
                cylinder_topology(*radius, *height, Point3::new(at[0], at[1], at[2]))
            }
            IrNode::Boolean { kind, a, b } => {
                let sa = solids
                    .get(a.0 as usize)
                    .and_then(|s| s.as_ref())
                    .ok_or_else(|| format!("missing node {}", a.0))?;
                let sb = solids
                    .get(b.0 as usize)
                    .and_then(|s| s.as_ref())
                    .ok_or_else(|| format!("missing node {}", b.0))?;
                let volume = match kind {
                    BooleanKind::Union => sa.volume_mm3 + sb.volume_mm3,
                    BooleanKind::Cut => (sa.volume_mm3 - sb.volume_mm3).max(0.0),
                    BooleanKind::Intersect => sa.volume_mm3.min(sb.volume_mm3),
                };
                SolidRec {
                    volume_mm3: volume,
                    centroid: sa.centroid,
                    faces: sa.faces.clone(),
                    edges: sa.edges.clone(),
                    vertices: sa.vertices.clone(),
                }
            }
            IrNode::Fillet { of, .. }
            | IrNode::Chamfer { of, .. }
            | IrNode::Label { of, .. }
            | IrNode::Translate { of, .. }
            | IrNode::Rotate { of, .. }
            | IrNode::Mirror { of, .. } => solids
                .get(of.0 as usize)
                .and_then(|s| s.as_ref())
                .ok_or_else(|| format!("missing node {}", of.0))?
                .clone(),
        };
        solids[idx] = Some(rec);
    }
    let root = solids
        .get(ir.root.0 as usize)
        .and_then(|s| s.as_ref())
        .ok_or("missing root")?
        .clone();
    Ok(TopologySnapshot::single_solid(root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent10_meets_target() {
        let root = default_tasks_root();
        assert!(
            root.is_dir(),
            "expected harness/tasks at {}",
            root.display()
        );
        let card = run_suite(SUITE_AGENT10, &RunOpts::default()).expect("suite");
        if !card.meets_target {
            for t in &card.tasks {
                if !t.ok {
                    eprintln!("FAIL {} — {}", t.id, t.detail);
                }
            }
        }
        assert!(
            card.meets_target,
            "score {}/10 < target {}",
            card.score_over_10, card.target
        );
        assert!(card.passed >= 6);
        assert_eq!(card.total, 10);
    }

    #[test]
    fn agent10_live_oracle_meets_target() {
        let root = default_tasks_root();
        let live = LiveOpts {
            // In-process oracle — no Python/shell (Windows CI safe).
            cmd: "@oracle".into(),
            timeout_secs: 60,
            part_rel: "part.cad.star".into(),
            snapshot: true,
        };
        let card = run_suite(
            SUITE_AGENT10,
            &RunOpts {
                tasks_root: Some(root),
                live: Some(live),
            },
        )
        .expect("live suite");
        if !card.meets_target {
            for t in &card.tasks {
                if !t.ok {
                    eprintln!("FAIL {} — {}", t.id, t.detail);
                }
            }
        }
        assert!(card.meets_target, "live score {}/10", card.score_over_10);
        assert_eq!(card.mode, "live");
        assert_eq!(card.total, 10);
    }

    #[test]
    fn live_model_id_oracle_and_env() {
        let prev = std::env::var("CADRION_HARNESS_MODEL_ID").ok();
        std::env::remove_var("CADRION_HARNESS_MODEL_ID");
        assert_eq!(live_model_id("@oracle"), "oracle:in-process");
        assert_eq!(
            live_model_id("python3 harness/drivers/oracle_agent.py"),
            "oracle:python"
        );
        assert_eq!(
            live_model_id("python3 harness/drivers/openai_starlark.py"),
            "external-cmd"
        );
        std::env::set_var("CADRION_HARNESS_MODEL_ID", "unit-test-model");
        assert_eq!(
            live_model_id("python3 harness/drivers/openai_starlark.py"),
            "unit-test-model"
        );
        match prev {
            Some(v) => std::env::set_var("CADRION_HARNESS_MODEL_ID", v),
            None => std::env::remove_var("CADRION_HARNESS_MODEL_ID"),
        }
    }
}
