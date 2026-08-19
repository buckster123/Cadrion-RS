//! Live external agent driver (`--cmd`).
//!
//! Protocol: for each task loop the harness spawns `cmd` with env:
//! - `CADRION_HARNESS_TASK_ID`, `CADRION_HARNESS_PROMPT`
//! - `CADRION_HARNESS_WORKDIR`, `CADRION_HARNESS_PART` (path to `part.cad.star`)
//! - `CADRION_HARNESS_LOOP`, `CADRION_HARNESS_MAX_LOOPS`
//! - `CADRION_HARNESS_TASK_FILE` (path to task JSON, read-only)
//!
//! The agent must leave a valid `.cad.star` at `CADRION_HARNESS_PART`. The harness then
//! builds + asserts success criteria (prompt-only to the agent — no step leaks).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cadrion_inspect::inspect_refs;
use cadrion_kernel::{GeomKernel, MockKernel};
use cadrion_lang::{evaluate, execute_ir, EvalOptions};
use cadrion_render::{mesh_from_ir, write_snapshot_packet, SnapshotOptions, ViewName};

use crate::runner::{topo_from_ir, LoopState};
use crate::scenario::{AssertSpec, Task};
use crate::score::TaskResult;

#[derive(Debug, Clone)]
pub struct LiveOpts {
    /// Shell command (run via `sh -c`).
    pub cmd: String,
    /// Per-loop timeout seconds (0 = wait forever).
    pub timeout_secs: u64,
    /// Relative part path under workdir.
    pub part_rel: String,
    /// Run software snapshot after successful build (for snapshot_ok asserts).
    pub snapshot: bool,
}

impl Default for LiveOpts {
    fn default() -> Self {
        Self {
            cmd: String::new(),
            timeout_secs: 300,
            part_rel: "part.cad.star".into(),
            snapshot: true,
        }
    }
}

pub fn run_task_live(task: &Task, task_file: &Path, live: &LiveOpts) -> Result<TaskResult, String> {
    let started = Instant::now();
    let work = std::env::temp_dir().join(format!(
        "cadrion-harness-live-{}-{}",
        task.id,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;

    fs::write(work.join("PROMPT.txt"), format!("{}\n", task.prompt)).map_err(|e| e.to_string())?;

    let success = task.success_assert().ok_or_else(|| {
        format!(
            "task {}: no success criteria (add asserts or success field)",
            task.id
        )
    })?;

    let mut last_err = String::from("no loops attempted");
    let max = task.max_loops.max(1);

    for loop_n in 1..=max {
        let part_path = work.join(&live.part_rel);
        let _ = fs::remove_file(&part_path);

        let agent_result = if live.cmd.trim() == "@oracle" {
            run_oracle_write(task_file, &part_path).map(|()| AgentMeta {
                status: 0,
                stderr_tail: "oracle:in-process".into(),
            })
        } else {
            run_agent_cmd(live, task, task_file, &work, &part_path, loop_n, max)
        };

        match agent_result {
            Ok(meta) => {
                if !part_path.is_file() {
                    last_err = format!(
                        "loop {loop_n}: agent exited {} but missing {}",
                        meta.status,
                        part_path.display()
                    );
                    continue;
                }
                match verify_part(&part_path, &work, &success, live.snapshot) {
                    Ok(()) => {
                        let detail = format!(
                            "live ok loop {loop_n}/{max}; agent exit {}; {}",
                            meta.status,
                            truncate(&meta.stderr_tail, 120)
                        );
                        let _ = fs::remove_dir_all(&work);
                        return Ok(TaskResult {
                            id: task.id.clone(),
                            ok: true,
                            loops_used: loop_n,
                            max_loops: max,
                            wall_ms: started.elapsed().as_millis() as u64,
                            detail,
                            prompt: task.prompt.clone(),
                            mode: "live".into(),
                        });
                    }
                    Err(e) => {
                        last_err = format!("loop {loop_n}: verify failed: {e}");
                    }
                }
            }
            Err(e) => {
                last_err = format!("loop {loop_n}: {e}");
            }
        }
    }

    let _ = fs::remove_dir_all(&work);
    Ok(TaskResult {
        id: task.id.clone(),
        ok: false,
        loops_used: max,
        max_loops: max,
        wall_ms: started.elapsed().as_millis() as u64,
        detail: last_err,
        prompt: task.prompt.clone(),
        mode: "live".into(),
    })
}

struct AgentMeta {
    status: i32,
    stderr_tail: String,
}

/// In-process oracle (CI): copy last-loop write from task JSON → part path.
fn run_oracle_write(task_file: &Path, part_path: &Path) -> Result<(), String> {
    let text = fs::read_to_string(task_file).map_err(|e| e.to_string())?;
    let task: Task = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let loops = task.loops;
    let last = loops
        .last()
        .ok_or_else(|| "oracle: task has no loops".to_string())?;
    let mut content = None;
    let mut rel = "part.cad.star".to_string();
    for step in last {
        if let crate::scenario::Step::Write { path, content: c } = step {
            rel = path.clone();
            content = Some(c.clone());
        }
    }
    let content = content.ok_or_else(|| "oracle: no write in last loop".to_string())?;
    let out = if part_path.as_os_str().is_empty() {
        PathBuf::from(&rel)
    } else {
        part_path.to_path_buf()
    };
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&out, content).map_err(|e| format!("oracle write: {e}"))?;
    let _ = rel;
    Ok(())
}

fn run_agent_cmd(
    live: &LiveOpts,
    task: &Task,
    task_file: &Path,
    work: &Path,
    part_path: &Path,
    loop_n: u32,
    max: u32,
) -> Result<AgentMeta, String> {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&live.cmd);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(&live.cmd);
        c
    };
    cmd.env("CADRION_HARNESS_TASK_ID", &task.id)
        .env("CADRION_HARNESS_PROMPT", &task.prompt)
        .env("CADRION_HARNESS_WORKDIR", work)
        .env("CADRION_HARNESS_PART", part_path)
        .env("CADRION_HARNESS_LOOP", loop_n.to_string())
        .env("CADRION_HARNESS_MAX_LOOPS", max.to_string())
        .env("CADRION_HARNESS_TASK_FILE", task_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn().map_err(|e| format!("spawn cmd: {e}"))?;
    let output = if live.timeout_secs == 0 {
        child.wait_with_output().map_err(|e| format!("wait: {e}"))?
    } else {
        wait_timeout(child, Duration::from_secs(live.timeout_secs))?
    };

    let status = output.status.code().unwrap_or(-1);
    let stderr_tail = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout_tail = String::from_utf8_lossy(&output.stdout).into_owned();
    let combined = if stderr_tail.is_empty() {
        stdout_tail
    } else {
        format!("{stderr_tail}\n{stdout_tail}")
    };

    if status != 0 {
        return Err(format!("agent exit {status}: {}", truncate(&combined, 200)));
    }
    Ok(AgentMeta {
        status,
        stderr_tail: combined,
    })
}

fn wait_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("agent timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("try_wait: {e}")),
        }
    }
}

fn verify_part(
    part_path: &Path,
    work: &Path,
    success: &AssertSpec,
    do_snapshot: bool,
) -> Result<(), String> {
    let src = fs::read_to_string(part_path).map_err(|e| format!("read part: {e}"))?;
    let name = part_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("part.cad.star");
    let eval = evaluate(&src, &EvalOptions::new(name));
    if !eval.ok {
        return Err(format!("eval failed: {:?}", eval.diagnostics));
    }
    let ir = eval.ir.ok_or("missing ir")?;
    let mut k = MockKernel::new();
    let sid = execute_ir(&mut k, &ir).map_err(|e| e.to_string())?;
    let facts = k.facts(sid).map_err(|e| e.to_string())?;
    let snap = topo_from_ir(&ir)?;

    let mut st = LoopState {
        last_facts: Some(facts),
        last_ir: Some(ir.clone()),
        last_label: ir.label.clone(),
        last_snap: Some(snap),
        snapshot_ok: false,
        work: work.to_path_buf(),
    };

    if do_snapshot || success.snapshot_ok {
        let (mesh, _) = mesh_from_ir(&ir).map_err(|e| format!("mesh: {e}"))?;
        let out = work.join("snap_out");
        fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let opts = SnapshotOptions {
            views: vec![ViewName::Iso],
            width: 64,
            height: 64,
            gif: false,
            gif_frames: 1,
            gif_delay_cs: 6,
            notes: vec!["live-harness".into()],
        };
        let r = write_snapshot_packet(&mesh, &out, &opts).map_err(|e| format!("snapshot: {e}"))?;
        if r.manifest.views.is_empty() {
            return Err("snapshot produced no views".into());
        }
        st.snapshot_ok = true;
    }

    apply_success(success, &st)
}

fn apply_success(a: &AssertSpec, st: &LoopState) -> Result<(), String> {
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
        return Err("assert volume: no facts".into());
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
        return Err("snapshot_ok required".into());
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        format!("{}…", &t[..max])
    }
}
