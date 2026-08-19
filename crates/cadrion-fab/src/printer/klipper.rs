//! Klipper / Moonraker printer adapter — same consent gates as Bambu.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde_json::json;

use super::{
    hex_sha256, DryRunReport, Printer, PrinterError, PrinterInfo, StartGate, StartRequest,
    CONFIRM_START,
};
use crate::gcode::{check_gcode, PrinterVolume};

/// Moonraker side effects (upload + print start). Injected for tests.
pub trait MoonrakerTransport: Send + Sync {
    fn upload_gcode(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        remote_name: &str,
        bytes: &[u8],
    ) -> Result<(), PrinterError>;

    fn start_print(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        remote_name: &str,
    ) -> Result<(), PrinterError>;
}

#[derive(Debug, Default, Clone)]
pub struct NullMoonrakerTransport;

impl MoonrakerTransport for NullMoonrakerTransport {
    fn upload_gcode(
        &self,
        _base_url: &str,
        _api_key: Option<&str>,
        _remote_name: &str,
        _bytes: &[u8],
    ) -> Result<(), PrinterError> {
        Err(PrinterError::Transport(
            "null moonraker transport: live I/O disabled".into(),
        ))
    }

    fn start_print(
        &self,
        _base_url: &str,
        _api_key: Option<&str>,
        _remote_name: &str,
    ) -> Result<(), PrinterError> {
        Err(PrinterError::Transport(
            "null moonraker transport: live I/O disabled".into(),
        ))
    }
}

#[derive(Debug, Default, Clone)]
pub struct RecordingMoonrakerTransport {
    pub log: Arc<Mutex<Vec<String>>>,
}

impl RecordingMoonrakerTransport {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn entries(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl MoonrakerTransport for RecordingMoonrakerTransport {
    fn upload_gcode(
        &self,
        base_url: &str,
        _api_key: Option<&str>,
        remote_name: &str,
        bytes: &[u8],
    ) -> Result<(), PrinterError> {
        self.log.lock().unwrap().push(format!(
            "moonraker_upload base={base_url} remote={remote_name} bytes={}",
            bytes.len()
        ));
        Ok(())
    }

    fn start_print(
        &self,
        base_url: &str,
        _api_key: Option<&str>,
        remote_name: &str,
    ) -> Result<(), PrinterError> {
        self.log.lock().unwrap().push(format!(
            "moonraker_start base={base_url} file={remote_name}"
        ));
        Ok(())
    }
}

/// Live Moonraker via `curl` (multipart upload + print/start).
#[derive(Debug, Clone)]
pub struct ExternalMoonrakerTransport {
    pub curl: PathBuf,
    pub timeout_secs: u64,
}

impl Default for ExternalMoonrakerTransport {
    fn default() -> Self {
        Self {
            curl: PathBuf::from(
                cadrion_kernel::env_var("CADRION_CURL").unwrap_or_else(|_| "curl".into()),
            ),
            timeout_secs: 60,
        }
    }
}

impl ExternalMoonrakerTransport {
    pub fn detect() -> Result<Self, PrinterError> {
        let t = Self::default();
        which_ok(&t.curl).map_err(|e| {
            PrinterError::Transport(format!(
                "curl not found ({e}); install curl or set CADRION_CURL"
            ))
        })?;
        Ok(t)
    }
}

fn which_ok(bin: &Path) -> Result<(), String> {
    if bin.is_file() {
        return Ok(());
    }
    let name = bin.file_name().and_then(|s| s.to_str()).unwrap_or("curl");
    let path = std::env::var_os("PATH").ok_or_else(|| "PATH unset".to_string())?;
    for dir in std::env::split_paths(&path) {
        if dir.join(name).is_file() {
            return Ok(());
        }
    }
    Err(format!("{name} not on PATH"))
}

impl MoonrakerTransport for ExternalMoonrakerTransport {
    fn upload_gcode(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        remote_name: &str,
        bytes: &[u8],
    ) -> Result<(), PrinterError> {
        let tmp = std::env::temp_dir().join(format!(
            "cadrion-klipper-{}",
            remote_name.replace(['/', '\\', ' '], "_")
        ));
        std::fs::write(&tmp, bytes)
            .map_err(|e| PrinterError::Transport(format!("write temp gcode: {e}")))?;

        let url = format!("{}/server/files/upload", base_url.trim_end_matches('/'));
        let mut cmd = Command::new(&self.curl);
        cmd.arg("-sS")
            .arg("--fail")
            .arg("--max-time")
            .arg(self.timeout_secs.to_string())
            .arg("-F")
            .arg(format!("file=@{};filename={}", tmp.display(), remote_name))
            .arg("-F")
            .arg("root=gcodes")
            .arg(&url);
        if let Some(k) = api_key {
            if !k.is_empty() {
                cmd.arg("-H").arg(format!("X-Api-Key: {k}"));
            }
        }
        let out = cmd
            .output()
            .map_err(|e| PrinterError::Transport(format!("spawn curl upload: {e}")))?;
        let _ = std::fs::remove_file(&tmp);
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            return Err(PrinterError::Transport(format!(
                "moonraker upload failed: status={:?} stderr={stderr} stdout={stdout}",
                out.status.code()
            )));
        }
        Ok(())
    }

    fn start_print(
        &self,
        base_url: &str,
        api_key: Option<&str>,
        remote_name: &str,
    ) -> Result<(), PrinterError> {
        // Moonraker: POST /printer/print/start?filename=<path relative to gcodes>
        let url = format!(
            "{}/printer/print/start?filename={}",
            base_url.trim_end_matches('/'),
            urlencoding_minimal(remote_name)
        );
        let mut cmd = Command::new(&self.curl);
        cmd.arg("-sS")
            .arg("--fail")
            .arg("--max-time")
            .arg(self.timeout_secs.to_string())
            .arg("-X")
            .arg("POST")
            .arg(&url);
        if let Some(k) = api_key {
            if !k.is_empty() {
                cmd.arg("-H").arg(format!("X-Api-Key: {k}"));
            }
        }
        let out = cmd
            .output()
            .map_err(|e| PrinterError::Transport(format!("spawn curl start: {e}")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            return Err(PrinterError::Transport(format!(
                "moonraker print/start failed: status={:?} stderr={stderr} stdout={stdout}",
                out.status.code()
            )));
        }
        Ok(())
    }
}

fn urlencoding_minimal(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                o.push(b as char)
            }
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

/// Klipper via Moonraker HTTP API.
///
/// **Safety:** identical gate story to Bambu — allowlist + sha256 + `confirm=START`
/// + gcode-check; network only with `live` + transport.
pub struct KlipperAdapter {
    info: PrinterInfo,
    /// Base URL e.g. `http://192.168.1.60:7125`
    base_url: String,
    api_key: Option<String>,
    transport: Arc<dyn MoonrakerTransport>,
}

impl std::fmt::Debug for KlipperAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlipperAdapter")
            .field("info", &self.info)
            .field("base_url", &self.base_url)
            .field("has_api_key", &self.api_key.is_some())
            .finish()
    }
}

impl KlipperAdapter {
    pub fn new(id: impl Into<String>, host: impl Into<String>, model: impl Into<String>) -> Self {
        let host = host.into();
        let base_url = if host.starts_with("http://") || host.starts_with("https://") {
            host.clone()
        } else if host.contains(':') {
            format!("http://{host}")
        } else {
            format!("http://{host}:7125")
        };
        Self {
            info: PrinterInfo {
                id: id.into(),
                model: model.into(),
                host,
                allowlisted: false,
                serial: None,
            },
            base_url,
            api_key: None,
            transport: Arc::new(NullMoonrakerTransport),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn with_allowlisted(mut self, yes: bool) -> Self {
        self.info.allowlisted = yes;
        self
    }

    pub fn with_transport(mut self, t: Arc<dyn MoonrakerTransport>) -> Self {
        self.transport = t;
        self
    }

    pub fn from_env(
        id: impl Into<String>,
        host: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Self {
        let key = api_key.or_else(|| cadrion_kernel::env_var("CADRION_MOONRAKER_API_KEY").ok());
        let mut a = Self::new(id, host, model);
        if let Some(u) = base_url.or_else(|| cadrion_kernel::env_var("CADRION_MOONRAKER_URL").ok())
        {
            a = a.with_base_url(u);
        }
        if let Some(k) = key {
            a = a.with_api_key(k);
        }
        a
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Printer for KlipperAdapter {
    fn info(&self) -> &PrinterInfo {
        &self.info
    }

    fn status(&self) -> Result<serde_json::Value, PrinterError> {
        Ok(json!({
            "ok": true,
            "backend": "klipper",
            "printer": self.info,
            "moonraker_url": self.base_url,
            "mode": if self.api_key.is_some() { "api_key_present" } else { "no_api_key" },
            "state": "unknown_local",
            "note": "Status is local metadata only; live Moonraker poll not required for dry-run.",
            "gates": {
                "allowlist": "required for start",
                "sha256": "must match dry-run",
                "confirm": CONFIRM_START,
                "live": "requires --live after gates"
            }
        }))
    }

    fn dry_run(
        &self,
        gcode_path: &Path,
        volume: &PrinterVolume,
    ) -> Result<DryRunReport, PrinterError> {
        let bytes =
            std::fs::read(gcode_path).map_err(|e| PrinterError::Msg(format!("read gcode: {e}")))?;
        let sha = hex_sha256(&bytes);
        let text = String::from_utf8_lossy(&bytes);
        let gcode_check = check_gcode(&text, volume);
        let mut notes = vec![
            "dry-run only: no Moonraker upload performed".into(),
            format!("target {}/server/files/upload (not contacted)", self.base_url),
            format!(
                "to live-start later: --backend klipper --sha256 {sha} --confirm {CONFIRM_START} --allowlist {} --live",
                self.info.id
            ),
        ];
        if self.api_key.is_none() {
            notes.push(
                "no API key set (CADRION_MOONRAKER_API_KEY or --api-key) — optional if Moonraker open"
                    .into(),
            );
        }
        if !gcode_check.ok {
            notes.push("gcode-check failed — upload would be refused".into());
        }
        Ok(DryRunReport {
            ok: gcode_check.ok,
            printer_id: self.info.id.clone(),
            gcode_sha256: sha,
            gcode_check,
            would_upload_to: format!("{}/server/files/upload", self.base_url),
            notes,
        })
    }

    fn start(
        &self,
        req: &StartRequest,
        allowlist: &BTreeSet<String>,
    ) -> Result<StartGate, PrinterError> {
        let mut errors = Vec::new();
        if req.confirm != CONFIRM_START {
            errors.push(format!(
                "confirm must be exactly \"{CONFIRM_START}\" (got {:?})",
                req.confirm
            ));
        }
        if req.printer_id != self.info.id {
            errors.push(format!(
                "printer_id mismatch: req={} adapter={}",
                req.printer_id, self.info.id
            ));
        }
        if !allowlist.contains(&req.printer_id) && !self.info.allowlisted {
            errors.push(format!(
                "printer '{}' not on allow-list (pass --allowlist {})",
                req.printer_id, req.printer_id
            ));
        }
        let path = Path::new(&req.gcode_path);
        let bytes =
            std::fs::read(path).map_err(|e| PrinterError::Msg(format!("read gcode: {e}")))?;
        let sha = hex_sha256(&bytes);
        if !sha.eq_ignore_ascii_case(&req.gcode_sha256) {
            errors.push(format!(
                "gcode hash mismatch: file={sha} req={}",
                req.gcode_sha256
            ));
        }
        let report = check_gcode(&String::from_utf8_lossy(&bytes), &PrinterVolume::default());
        if !report.ok {
            errors.push(format!("gcode-check failed: {:?}", report.errors));
        }

        if !errors.is_empty() {
            return Ok(StartGate {
                ok: false,
                errors,
                live_attempted: false,
                uploaded_as: None,
                mqtt_topic: None,
                notes: None,
            });
        }

        if !req.live {
            return Ok(StartGate {
                ok: false,
                errors: vec![
                    "gates passed, but live I/O not requested — re-run with --live to contact Moonraker"
                        .into(),
                ],
                live_attempted: false,
                uploaded_as: None,
                mqtt_topic: None,
                notes: Some(vec![
                    "safety: default start is dry (no network) even when gates pass".into(),
                    format!("hash ok: {sha}"),
                    format!("moonraker: {}", self.base_url),
                ]),
            });
        }

        let remote = req.remote_name.clone().unwrap_or_else(|| {
            Path::new(&req.gcode_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("cadrion-job.gcode")
                .to_string()
        });
        let key = self.api_key.as_deref();
        let mut notes = vec!["gates passed; attempting Moonraker upload + print/start".into()];

        if let Err(e) = self
            .transport
            .upload_gcode(&self.base_url, key, &remote, &bytes)
        {
            return Ok(StartGate {
                ok: false,
                errors: vec![e.to_string()],
                live_attempted: true,
                uploaded_as: None,
                mqtt_topic: None,
                notes: Some(notes),
            });
        }
        notes.push(format!("upload ok → {} gcodes/{remote}", self.base_url));

        if let Err(e) = self.transport.start_print(&self.base_url, key, &remote) {
            return Ok(StartGate {
                ok: false,
                errors: vec![format!("upload ok but print/start failed: {e}")],
                live_attempted: true,
                uploaded_as: Some(remote),
                mqtt_topic: None,
                notes: Some(notes),
            });
        }
        notes.push(format!("print/start issued for {remote}"));
        Ok(StartGate {
            ok: true,
            errors: vec![],
            live_attempted: true,
            uploaded_as: Some(remote),
            mqtt_topic: None,
            notes: Some(notes),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn dry_run_hashes() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let p = KlipperAdapter::new("klipper:ender", "192.168.1.60", "ender3");
        let r = p.dry_run(f.path(), &PrinterVolume::default()).unwrap();
        assert_eq!(r.gcode_sha256.len(), 64);
        assert!(r.ok);
        assert!(r.would_upload_to.contains("7125"));
    }

    #[test]
    fn start_without_live_refuses_network() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let sha = hex_sha256(&std::fs::read(f.path()).unwrap());
        let rec = RecordingMoonrakerTransport::new();
        let p = KlipperAdapter::new("klipper:ender", "192.168.1.60", "ender3")
            .with_transport(Arc::new(rec.clone()));
        let mut allow = BTreeSet::new();
        allow.insert("klipper:ender".into());
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "klipper:ender".into(),
                    gcode_path: f.path().display().to_string(),
                    gcode_sha256: sha,
                    confirm: CONFIRM_START.into(),
                    live: false,
                    remote_name: None,
                },
                &allow,
            )
            .unwrap();
        assert!(!gate.ok);
        assert!(!gate.live_attempted);
        assert!(rec.entries().is_empty());
    }

    #[test]
    fn start_live_calls_transport() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let sha = hex_sha256(&std::fs::read(f.path()).unwrap());
        let rec = RecordingMoonrakerTransport::new();
        let p = KlipperAdapter::new("klipper:ender", "192.168.1.60", "ender3")
            .with_transport(Arc::new(rec.clone()));
        let mut allow = BTreeSet::new();
        allow.insert("klipper:ender".into());
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "klipper:ender".into(),
                    gcode_path: f.path().display().to_string(),
                    gcode_sha256: sha,
                    confirm: CONFIRM_START.into(),
                    live: true,
                    remote_name: Some("job.gcode".into()),
                },
                &allow,
            )
            .unwrap();
        assert!(gate.ok, "{:?}", gate.errors);
        assert!(gate.live_attempted);
        let log = rec.entries();
        assert!(log.iter().any(|e| e.starts_with("moonraker_upload")));
        assert!(log.iter().any(|e| e.starts_with("moonraker_start")));
    }

    #[test]
    fn live_without_allowlist_silent() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let sha = hex_sha256(&std::fs::read(f.path()).unwrap());
        let rec = RecordingMoonrakerTransport::new();
        let p = KlipperAdapter::new("klipper:ender", "192.168.1.60", "ender3")
            .with_transport(Arc::new(rec.clone()));
        let allow = BTreeSet::new();
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "klipper:ender".into(),
                    gcode_path: f.path().display().to_string(),
                    gcode_sha256: sha,
                    confirm: CONFIRM_START.into(),
                    live: true,
                    remote_name: None,
                },
                &allow,
            )
            .unwrap();
        assert!(!gate.ok);
        assert!(!gate.live_attempted);
        assert!(rec.entries().is_empty());
    }
}
