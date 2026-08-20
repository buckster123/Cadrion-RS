//! HTTP routes.

use std::convert::Infallible;
use std::path::PathBuf;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt as _;
use tower_http::cors::CorsLayer;

use crate::jobs::{Job, JobEvent, JobStatus};
use crate::openapi::openapi_doc;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/openapi.json", get(openapi))
        .route("/v1/build", post(build))
        .route("/v1/inspect/refs", post(inspect_refs))
        .route("/v1/inspect/measure", post(inspect_measure))
        .route("/v1/inspect/dims", post(inspect_dims))
        .route("/v1/inspect/align", post(inspect_align))
        .route("/v1/inspect/frame", post(inspect_frame))
        .route("/v1/inspect/diff", post(inspect_diff))
        .route("/v1/export", post(export))
        .route("/v1/sdf/sample", post(sdf_sample))
        .route("/v1/snapshot", post(snapshot))
        .route("/v1/parts/search", post(parts_search))
        .route("/v1/assembly/validate", post(assembly_validate))
        .route("/v1/jobs", post(create_job))
        .route("/v1/jobs/{id}", get(get_job))
        .route("/v1/jobs/{id}/events", get(job_events))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health(State(st): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "service": "cadrion-api",
        "version": crate::VERSION,
        "bind": st.cfg.bind,
    }))
}

async fn openapi() -> impl IntoResponse {
    Json(openapi_doc())
}

fn auth(st: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(expected) = st.cfg.token.as_ref() else {
        return Ok(());
    };
    let got = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let token = got
        .strip_prefix("Bearer ")
        .or_else(|| got.strip_prefix("bearer "))
        .unwrap_or("");
    if token == expected {
        Ok(())
    } else {
        Err(ApiError::unauthorized("invalid or missing bearer token"))
    }
}

#[derive(Debug, Deserialize)]
struct PathBody {
    path: String,
    #[serde(default)]
    set: Option<Value>,
    #[serde(default)]
    facts: Option<bool>,
    #[serde(default)]
    views: Option<String>,
    #[serde(default)]
    size: Option<u32>,
    #[serde(default)]
    include_images: Option<bool>,
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    parts_root: Option<String>,
}

async fn build(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PathBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    let mut args = json!({"path": path});
    if let Some(set) = body.set {
        args["set"] = set;
    }
    call_tool("build", &args)
}

async fn inspect_refs(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PathBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    let args = json!({
        "path": path,
        "facts": body.facts.unwrap_or(true)
    });
    call_tool("inspect_refs", &args)
}

#[derive(Debug, Deserialize)]
struct MeasureBody {
    path: String,
    a: String,
    #[serde(default)]
    b: Option<String>,
    kind: String,
}

async fn inspect_measure(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<MeasureBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    let mut args = json!({"path": path, "a": body.a, "kind": body.kind});
    if let Some(b) = body.b {
        args["b"] = json!(b);
    }
    call_tool("measure", &args)
}

#[derive(Debug, Deserialize)]
struct DimsBody {
    path: String,
    #[serde(default)]
    out: Option<String>,
    #[serde(default)]
    dims: Option<Value>,
}

async fn inspect_dims(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DimsBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    let mut args = json!({"path": path});
    if let Some(out) = body.out {
        args["out"] = json!(resolve_out_path(&st, &out));
    }
    if let Some(dims) = body.dims {
        args["dims"] = dims;
    }
    call_tool("inspect_dims", &args)
}

#[derive(Debug, Deserialize)]
struct AlignBody {
    path: String,
    a: String,
    b: String,
    #[serde(default)]
    expect: Option<String>,
    #[serde(default)]
    distance: Option<f64>,
    #[serde(default)]
    tol: Option<f64>,
    #[serde(default)]
    tol_deg: Option<f64>,
}

async fn inspect_align(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AlignBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    let mut args = json!({"path": path, "a": body.a, "b": body.b});
    if let Some(expect) = body.expect {
        args["expect"] = json!(expect);
    }
    if let Some(distance) = body.distance {
        args["distance"] = json!(distance);
    }
    if let Some(tol) = body.tol {
        args["tol"] = json!(tol);
    }
    if let Some(tol_deg) = body.tol_deg {
        args["tol_deg"] = json!(tol_deg);
    }
    call_tool("align_check", &args)
}

#[derive(Debug, Deserialize)]
struct FrameBody {
    path: String,
    selector: String,
}

async fn inspect_frame(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<FrameBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    call_tool("frame", &json!({"path": path, "selector": body.selector}))
}

#[derive(Debug, Deserialize)]
struct DiffBody {
    old: String,
    new: String,
}

async fn inspect_diff(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<DiffBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let old = resolve_path(&st, &body.old)?;
    let new = resolve_path(&st, &body.new)?;
    call_tool("diff", &json!({"old": old, "new": new}))
}

#[derive(Debug, Deserialize)]
struct ExportBody {
    path: String,
    format: String,
    #[serde(default)]
    out: Option<String>,
}

async fn export(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ExportBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    let mut args = json!({"path": path, "format": body.format});
    if let Some(out) = body.out {
        args["out"] = json!(resolve_out_path(&st, &out));
    }
    call_tool("export", &args)
}

#[derive(Debug, Deserialize)]
struct SdfBody {
    prim: String,
    a: f64,
    b: f64,
    #[serde(default)]
    c: Option<f64>,
    #[serde(default)]
    res: Option<u64>,
    #[serde(default)]
    pad: Option<f64>,
    #[serde(default)]
    out: Option<String>,
    #[serde(default)]
    stem: Option<String>,
}

async fn sdf_sample(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SdfBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let mut args = json!({"prim": body.prim, "a": body.a, "b": body.b});
    if let Some(c) = body.c {
        args["c"] = json!(c);
    }
    if let Some(res) = body.res {
        args["res"] = json!(res);
    }
    if let Some(pad) = body.pad {
        args["pad"] = json!(pad);
    }
    if let Some(stem) = body.stem {
        args["stem"] = json!(stem);
    }
    let out = body
        .out
        .map(|p| resolve_out_path(&st, &p))
        .unwrap_or_else(|| st.cfg.project_root.join("sdf_out").display().to_string());
    args["out"] = json!(out);
    call_tool("sdf_sample", &args)
}

async fn snapshot(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PathBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let path = resolve_path(&st, &body.path)?;
    let args = json!({
        "path": path,
        "views": body.views.unwrap_or_else(|| "iso,front,top,right".into()),
        "size": body.size.unwrap_or(256),
        "include_images": body.include_images.unwrap_or(false)
    });
    call_tool("snapshot", &args)
}

async fn parts_search(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PathBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    use cadrion_parts::{LocalFsProvider, PartProvider};
    let root = body
        .parts_root
        .map(PathBuf::from)
        .unwrap_or_else(|| st.cfg.project_root.join("parts"));
    let prov = LocalFsProvider::new(root);
    let q = body.query.unwrap_or_default();
    let hits = prov.search(&q).map_err(|e| ApiError::bad(e.to_string()))?;
    Ok(Json(
        json!({"ok": true, "provider": prov.id(), "results": hits}),
    ))
}

#[derive(Debug, Deserialize)]
struct AssemblyBody {
    /// Path to assembly JSON relative to project root.
    path: String,
    /// Path to parts.lock (default project_root/parts.lock).
    #[serde(default)]
    lock: Option<String>,
}

async fn assembly_validate(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<AssemblyBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let assy_path = resolve_path_buf(&st, &body.path)?;
    let text = std::fs::read_to_string(&assy_path).map_err(|e| ApiError::bad(e.to_string()))?;
    let assy: cadrion_parts::AssemblySpec =
        serde_json::from_str(&text).map_err(|e| ApiError::bad(format!("assembly json: {e}")))?;

    let lock_path = body
        .lock
        .map(|p| resolve_path_buf(&st, &p))
        .transpose()?
        .unwrap_or_else(|| st.cfg.project_root.join("parts.lock"));

    let mut lock_ok = Vec::new();
    let mut lock_err = Vec::new();
    if lock_path.is_file() {
        let lock =
            cadrion_parts::load_parts_lock(&lock_path).map_err(|e| ApiError::bad(e.to_string()))?;
        for c in &assy.components {
            if c.from_lock {
                match cadrion_parts::verify_lock_entry(&lock, &c.source, &st.cfg.project_root) {
                    Ok(()) => lock_ok.push(c.source.clone()),
                    Err(e) => lock_err.push(format!("{}: {e}", c.source)),
                }
            }
        }
    } else {
        for c in &assy.components {
            if c.from_lock {
                lock_err.push(format!(
                    "{}: parts.lock missing at {}",
                    c.source,
                    lock_path.display()
                ));
            }
        }
    }

    let ok = lock_err.is_empty();
    Ok(Json(json!({
        "ok": ok,
        "assembly": assy.name,
        "components": assy.components.len(),
        "joints": assy.joints.len(),
        "lock_verified": lock_ok,
        "lock_errors": lock_err,
        "fail_closed": !ok && assy.components.iter().any(|c| c.from_lock),
    })))
}

#[derive(Debug, Deserialize)]
struct CreateJobBody {
    kind: String,
    #[serde(default)]
    payload: Value,
}

async fn create_job(
    State(st): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateJobBody>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let job = Job::new(body.kind.clone());
    let id = job.id.clone();
    {
        let mut jobs = st.jobs.write().await;
        jobs.insert(id.clone(), job.clone());
    }
    let job_out = job;
    let _ = st.events.send(JobEvent {
        job_id: id.clone(),
        status: JobStatus::Pending,
        message: Some("queued".into()),
    });

    // spawn worker
    let st2 = st.clone();
    let payload = body.payload;
    let kind = body.kind;
    tokio::spawn(async move {
        run_job(st2, id, kind, payload).await;
    });

    Ok(Json(json!({"ok": true, "job": job_out})))
}

async fn run_job(st: AppState, id: String, kind: String, payload: Value) {
    {
        let mut jobs = st.jobs.write().await;
        if let Some(j) = jobs.get_mut(&id) {
            j.status = JobStatus::Running;
        }
    }
    let _ = st.events.send(JobEvent {
        job_id: id.clone(),
        status: JobStatus::Running,
        message: Some("running".into()),
    });

    let result = match kind.as_str() {
        "build" | "inspect_refs" | "snapshot" | "measure" | "inspect_dims" | "align_check"
        | "frame" | "export" => {
            let path = payload.get("path").and_then(|p| p.as_str()).map(|p| {
                if std::path::Path::new(p).is_absolute() {
                    p.to_string()
                } else {
                    st.cfg.project_root.join(p).display().to_string()
                }
            });
            match path {
                Some(path) => {
                    let mut args = payload.clone();
                    args["path"] = json!(path);
                    if kind == "snapshot" && args.get("include_images").is_none() {
                        args["include_images"] = json!(false);
                    }
                    match cadrion_mcp::tools_call_for_api(&kind, &args) {
                        Ok(v) => Ok(v),
                        Err(e) => Err(e),
                    }
                }
                None => Err("missing path".into()),
            }
        }
        "diff" => {
            let mut args = payload.clone();
            for key in ["old", "new"] {
                if let Some(p) = args.get(key).and_then(|v| v.as_str()) {
                    args[key] = json!(resolve_out_path(&st, p));
                }
            }
            if args.get("old").and_then(|v| v.as_str()).is_none()
                || args.get("new").and_then(|v| v.as_str()).is_none()
            {
                Err("missing old/new".into())
            } else {
                cadrion_mcp::tools_call_for_api("diff", &args)
            }
        }
        "sdf_sample" => {
            let mut args = payload.clone();
            if let Some(out) = args.get("out").and_then(|v| v.as_str()) {
                args["out"] = json!(resolve_out_path(&st, out));
            }
            cadrion_mcp::tools_call_for_api("sdf_sample", &args)
        }
        other => Err(format!("unsupported job kind: {other}")),
    };

    let mut jobs = st.jobs.write().await;
    if let Some(j) = jobs.get_mut(&id) {
        match result {
            Ok(v) => {
                j.status = JobStatus::Completed;
                j.result = Some(v);
                let _ = st.events.send(JobEvent {
                    job_id: id.clone(),
                    status: JobStatus::Completed,
                    message: Some("done".into()),
                });
            }
            Err(e) => {
                j.status = JobStatus::Failed;
                j.error = Some(e);
                let _ = st.events.send(JobEvent {
                    job_id: id.clone(),
                    status: JobStatus::Failed,
                    message: j.error.clone(),
                });
            }
        }
    }
}

async fn get_job(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    auth(&st, &headers)?;
    let jobs = st.jobs.read().await;
    let job = jobs
        .get(&id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("job not found"))?;
    Ok(Json(json!({"ok": true, "job": job})))
}

async fn job_events(
    State(st): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    auth(&st, &headers)?;
    {
        let jobs = st.jobs.read().await;
        if !jobs.contains_key(&id) {
            return Err(ApiError::not_found("job not found"));
        }
    }
    let rx = st.events.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(move |msg| {
        let id = id.clone();
        match msg {
            Ok(ev) if ev.job_id == id => {
                let data = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".into());
                Some(Ok(Event::default().data(data)))
            }
            _ => None,
        }
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15))))
}

fn call_tool(name: &str, args: &Value) -> Result<Json<Value>, ApiError> {
    match cadrion_mcp::tools_call_for_api(name, args) {
        Ok(v) => Ok(Json(v)),
        Err(e) => Err(ApiError::bad(e)),
    }
}

fn resolve_path(st: &AppState, p: &str) -> Result<String, ApiError> {
    Ok(resolve_path_buf(st, p)?.display().to_string())
}

/// Output path that may not exist yet (dims / sdf writes).
fn resolve_out_path(st: &AppState, p: &str) -> String {
    let path = PathBuf::from(p);
    if path.is_absolute() {
        path.display().to_string()
    } else {
        st.cfg.project_root.join(path).display().to_string()
    }
}

fn resolve_path_buf(st: &AppState, p: &str) -> Result<PathBuf, ApiError> {
    let path = PathBuf::from(p);
    let full = if path.is_absolute() {
        path
    } else {
        st.cfg.project_root.join(path)
    };
    if !full.exists() {
        return Err(ApiError::bad(format!("not found: {}", full.display())));
    }
    Ok(full)
}

struct ApiError {
    status: StatusCode,
    msg: String,
}

impl ApiError {
    fn bad(m: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            msg: m.into(),
        }
    }
    fn unauthorized(m: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            msg: m.into(),
        }
    }
    fn not_found(m: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            msg: m.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({"ok": false, "error": self.msg});
        (self.status, Json(body)).into_response()
    }
}
