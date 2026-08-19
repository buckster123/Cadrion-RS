//! OctoPrint printer adapter — same consent gates as Bambu / Klipper.

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

/// OctoPrint side effects (upload + optional print). Injected for tests.
pub trait OctoPrintTransport: Send + Sync {
    fn upload_and_print(
        &self,
        base_url: &str,
        api_key: &str,
        remote_name: &str,
        bytes: &[u8],
        start_print: bool,
    ) -> Result<(), PrinterError>;
}

#[derive(Debug, Default, Clone)]
pub struct NullOctoPrintTransport;

impl OctoPrintTransport for NullOctoPrintTransport {
    fn upload_and_print(
        &self,
        _base_url: &str,
        _api_key: &str,
        _remote_name: &str,
        _bytes: &[u8],
        _start_print: bool,
    ) -> Result<(), PrinterError> {
        Err(PrinterError::Transport(
            "null octoprint transport: live I/O disabled".into(),
        ))
    }
}

#[derive(Debug, Default, Clone)]
pub struct RecordingOctoPrintTransport {
    pub log: Arc<Mutex<Vec<String>>>,
}

impl RecordingOctoPrintTransport {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn entries(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl OctoPrintTransport for RecordingOctoPrintTransport {
    fn upload_and_print(
        &self,
        base_url: &str,
        _api_key: &str,
        remote_name: &str,
        bytes: &[u8],
        start_print: bool,
    ) -> Result<(), PrinterError> {
        self.log.lock().unwrap().push(format!(
            "octoprint_upload base={base_url} remote={remote_name} bytes={} print={start_print}",
            bytes.len()
        ));
        Ok(())
    }
}

/// Live OctoPrint via `curl` multipart to `/api/files/local`.
#[derive(Debug, Clone)]
pub struct ExternalOctoPrintTransport {
    pub curl: PathBuf,
    pub timeout_secs: u64,
}

impl Default for ExternalOctoPrintTransport {
    fn default() -> Self {
        Self {
            curl: PathBuf::from(
                cadrion_kernel::env_var("CADRION_CURL").unwrap_or_else(|_| "curl".into()),
            ),
            timeout_secs: 60,
        }
    }
}

impl ExternalOctoPrintTransport {
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

impl OctoPrintTransport for ExternalOctoPrintTransport {
    fn upload_and_print(
        &self,
        base_url: &str,
        api_key: &str,
        remote_name: &str,
        bytes: &[u8],
        start_print: bool,
    ) -> Result<(), PrinterError> {
        let tmp = std::env::temp_dir().join(format!(
            "cadrion-octo-{}",
            remote_name.replace(['/', '\\', ' '], "_")
        ));
        std::fs::write(&tmp, bytes)
            .map_err(|e| PrinterError::Transport(format!("write temp gcode: {e}")))?;

        let url = format!("{}/api/files/local", base_url.trim_end_matches('/'));
        let mut cmd = Command::new(&self.curl);
        cmd.arg("-sS")
            .arg("--fail")
            .arg("--max-time")
            .arg(self.timeout_secs.to_string())
            .arg("-H")
            .arg(format!("X-Api-Key: {api_key}"))
            .arg("-F")
            .arg(format!("file=@{};filename={}", tmp.display(), remote_name))
            .arg("-F")
            .arg(format!(
                "select={}",
                if start_print { "true" } else { "false" }
            ))
            .arg("-F")
            .arg(format!(
                "print={}",
                if start_print { "true" } else { "false" }
            ))
            .arg(&url);
        let out = cmd
            .output()
            .map_err(|e| PrinterError::Transport(format!("spawn curl octoprint: {e}")))?;
        let _ = std::fs::remove_file(&tmp);
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            return Err(PrinterError::Transport(format!(
                "octoprint upload failed: status={:?} stderr={stderr} stdout={stdout}",
                out.status.code()
            )));
        }
        Ok(())
    }
}

/// OctoPrint HTTP API adapter.
///
/// **Safety:** allowlist + sha256 + `confirm=START` + gcode-check; network only with `live`.
pub struct OctoPrintAdapter {
    info: PrinterInfo,
    base_url: String,
    api_key: Option<String>,
    transport: Arc<dyn OctoPrintTransport>,
}

impl std::fmt::Debug for OctoPrintAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OctoPrintAdapter")
            .field("info", &self.info)
            .field("base_url", &self.base_url)
            .field("has_api_key", &self.api_key.is_some())
            .finish()
    }
}

impl OctoPrintAdapter {
    pub fn new(id: impl Into<String>, host: impl Into<String>, model: impl Into<String>) -> Self {
        let host = host.into();
        let base_url = if host.starts_with("http://") || host.starts_with("https://") {
            host.clone()
        } else if host.contains(':') {
            format!("http://{host}")
        } else {
            // Common OctoPi default
            format!("http://{host}")
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
            transport: Arc::new(NullOctoPrintTransport),
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

    pub fn with_transport(mut self, t: Arc<dyn OctoPrintTransport>) -> Self {
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
        let key = api_key.or_else(|| cadrion_kernel::env_var("CADRION_OCTOPRINT_API_KEY").ok());
        let mut a = Self::new(id, host, model);
        if let Some(u) = base_url.or_else(|| cadrion_kernel::env_var("CADRION_OCTOPRINT_URL").ok())
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

impl Printer for OctoPrintAdapter {
    fn info(&self) -> &PrinterInfo {
        &self.info
    }

    fn status(&self) -> Result<serde_json::Value, PrinterError> {
        Ok(json!({
            "ok": true,
            "backend": "octoprint",
            "printer": self.info,
            "octoprint_url": self.base_url,
            "mode": if self.api_key.is_some() { "api_key_present" } else { "no_api_key" },
            "state": "unknown_local",
            "note": "Status is local metadata only; live OctoPrint poll not required for dry-run.",
            "gates": {
                "allowlist": "required for start",
                "sha256": "must match dry-run",
                "confirm": CONFIRM_START,
                "live": "requires --live after gates",
                "api_key": "required for live (CADRION_OCTOPRINT_API_KEY)"
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
            "dry-run only: no OctoPrint upload performed".into(),
            format!(
                "target {}/api/files/local (not contacted)",
                self.base_url
            ),
            format!(
                "to live-start later: --backend octoprint --sha256 {sha} --confirm {CONFIRM_START} --allowlist {} --live",
                self.info.id
            ),
        ];
        if self.api_key.is_none() {
            notes.push(
                "no API key set (CADRION_OCTOPRINT_API_KEY or --api-key) — required for live"
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
            would_upload_to: format!("{}/api/files/local", self.base_url),
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
                    "gates passed, but live I/O not requested — re-run with --live to contact OctoPrint"
                        .into(),
                ],
                live_attempted: false,
                uploaded_as: None,
                mqtt_topic: None,
                notes: Some(vec![
                    "safety: default start is dry (no network) even when gates pass".into(),
                    format!("hash ok: {sha}"),
                    format!("octoprint: {}", self.base_url),
                ]),
            });
        }

        let key = match &self.api_key {
            Some(k) if !k.is_empty() => k.clone(),
            _ => {
                return Ok(StartGate {
                    ok: false,
                    errors: vec![
                        "live start needs API key (--api-key or CADRION_OCTOPRINT_API_KEY)".into(),
                    ],
                    live_attempted: false,
                    uploaded_as: None,
                    mqtt_topic: None,
                    notes: None,
                });
            }
        };

        let remote = req.remote_name.clone().unwrap_or_else(|| {
            Path::new(&req.gcode_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("cadrion-job.gcode")
                .to_string()
        });
        let mut notes = vec!["gates passed; attempting OctoPrint upload+print".into()];

        if let Err(e) = self.transport.upload_and_print(
            &self.base_url,
            &key,
            &remote,
            &bytes,
            true, // select + print
        ) {
            return Ok(StartGate {
                ok: false,
                errors: vec![e.to_string()],
                live_attempted: true,
                uploaded_as: None,
                mqtt_topic: None,
                notes: Some(notes),
            });
        }
        notes.push(format!(
            "upload+print ok → {} local/{remote}",
            self.base_url
        ));
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
        let p = OctoPrintAdapter::new("octoprint:pi", "192.168.1.70", "ender");
        let r = p.dry_run(f.path(), &PrinterVolume::default()).unwrap();
        assert_eq!(r.gcode_sha256.len(), 64);
        assert!(r.ok);
        assert!(r.would_upload_to.contains("api/files/local"));
    }

    #[test]
    fn start_without_live_refuses() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let sha = hex_sha256(&std::fs::read(f.path()).unwrap());
        let rec = RecordingOctoPrintTransport::new();
        let p = OctoPrintAdapter::new("octoprint:pi", "192.168.1.70", "ender")
            .with_api_key("KEY")
            .with_transport(Arc::new(rec.clone()));
        let mut allow = BTreeSet::new();
        allow.insert("octoprint:pi".into());
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "octoprint:pi".into(),
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
        let rec = RecordingOctoPrintTransport::new();
        let p = OctoPrintAdapter::new("octoprint:pi", "192.168.1.70", "ender")
            .with_api_key("KEY")
            .with_transport(Arc::new(rec.clone()));
        let mut allow = BTreeSet::new();
        allow.insert("octoprint:pi".into());
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "octoprint:pi".into(),
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
        assert!(rec
            .entries()
            .iter()
            .any(|e| e.starts_with("octoprint_upload")));
    }

    #[test]
    fn live_needs_api_key() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let sha = hex_sha256(&std::fs::read(f.path()).unwrap());
        let rec = RecordingOctoPrintTransport::new();
        let p = OctoPrintAdapter::new("octoprint:pi", "192.168.1.70", "ender")
            .with_transport(Arc::new(rec.clone()));
        let mut allow = BTreeSet::new();
        allow.insert("octoprint:pi".into());
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "octoprint:pi".into(),
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
        assert!(gate.errors.iter().any(|e| e.contains("API key")));
    }
}
