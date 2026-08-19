//! Printer adapters — Bambu + Klipper/Moonraker + OctoPrint; hard start gates; live opt-in.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::gcode::{check_gcode, GcodeReport, PrinterVolume};

mod klipper;
mod octoprint;
pub use klipper::{
    ExternalMoonrakerTransport, KlipperAdapter, MoonrakerTransport, NullMoonrakerTransport,
    RecordingMoonrakerTransport,
};
pub use octoprint::{
    ExternalOctoPrintTransport, NullOctoPrintTransport, OctoPrintAdapter, OctoPrintTransport,
    RecordingOctoPrintTransport,
};

#[derive(Debug, Error)]
pub enum PrinterError {
    #[error("{0}")]
    Msg(String),
    #[error("start gate failed: {0}")]
    Gate(String),
    #[error("live transport: {0}")]
    Transport(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInfo {
    pub id: String,
    pub model: String,
    pub host: String,
    pub allowlisted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunReport {
    pub ok: bool,
    pub printer_id: String,
    pub gcode_sha256: String,
    pub gcode_check: GcodeReport,
    pub would_upload_to: String,
    pub notes: Vec<String>,
}

/// Explicit start confirmation token (must be exactly `"START"`).
pub const CONFIRM_START: &str = "START";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRequest {
    pub printer_id: String,
    pub gcode_path: String,
    pub gcode_sha256: String,
    /// Must equal [`CONFIRM_START`].
    pub confirm: String,
    /// Opt-in to real network I/O after gates pass.
    #[serde(default)]
    pub live: bool,
    /// Remote filename on printer (default: basename of gcode_path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartGate {
    pub ok: bool,
    pub errors: Vec<String>,
    #[serde(default)]
    pub live_attempted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uploaded_as: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mqtt_topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

/// Side-effectful LAN ops (FTPS upload + MQTT print). Injected for tests.
pub trait BambuTransport: Send + Sync {
    fn ftps_upload(
        &self,
        host: &str,
        access_code: &str,
        remote_name: &str,
        bytes: &[u8],
    ) -> Result<(), PrinterError>;

    fn mqtt_start_print(
        &self,
        host: &str,
        serial: &str,
        access_code: &str,
        remote_name: &str,
    ) -> Result<(), PrinterError>;
}

/// No network — used for dry-run / default start without `--live`.
#[derive(Debug, Default, Clone)]
pub struct NullTransport;

impl BambuTransport for NullTransport {
    fn ftps_upload(
        &self,
        _host: &str,
        _access_code: &str,
        _remote_name: &str,
        _bytes: &[u8],
    ) -> Result<(), PrinterError> {
        Err(PrinterError::Transport(
            "null transport: live I/O disabled".into(),
        ))
    }

    fn mqtt_start_print(
        &self,
        _host: &str,
        _serial: &str,
        _access_code: &str,
        _remote_name: &str,
    ) -> Result<(), PrinterError> {
        Err(PrinterError::Transport(
            "null transport: live I/O disabled".into(),
        ))
    }
}

/// Records calls for unit tests (no sockets).
#[derive(Debug, Default, Clone)]
pub struct RecordingTransport {
    pub log: Arc<Mutex<Vec<String>>>,
}

impl RecordingTransport {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn entries(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl BambuTransport for RecordingTransport {
    fn ftps_upload(
        &self,
        host: &str,
        _access_code: &str,
        remote_name: &str,
        bytes: &[u8],
    ) -> Result<(), PrinterError> {
        self.log.lock().unwrap().push(format!(
            "ftps_upload host={host} remote={remote_name} bytes={}",
            bytes.len()
        ));
        Ok(())
    }

    fn mqtt_start_print(
        &self,
        host: &str,
        serial: &str,
        _access_code: &str,
        remote_name: &str,
    ) -> Result<(), PrinterError> {
        self.log.lock().unwrap().push(format!(
            "mqtt_start host={host} serial={serial} file={remote_name}"
        ));
        Ok(())
    }
}

/// Live LAN transport via `curl` (FTPS) + `mosquitto_pub` (MQTT).
///
/// Bambu LAN uses self-signed TLS; both tools are invoked with insecure TLS flags.
/// Requires binaries on PATH (or absolute paths via env).
#[derive(Debug, Clone)]
pub struct ExternalLiveTransport {
    pub curl: PathBuf,
    pub mosquitto_pub: PathBuf,
    /// Extra seconds for network ops.
    pub timeout_secs: u64,
}

impl Default for ExternalLiveTransport {
    fn default() -> Self {
        Self {
            curl: PathBuf::from(
                cadrion_kernel::env_var("CADRION_CURL").unwrap_or_else(|_| "curl".into()),
            ),
            mosquitto_pub: PathBuf::from(
                cadrion_kernel::env_var("CADRION_MOSQUITTO_PUB")
                    .unwrap_or_else(|_| "mosquitto_pub".into()),
            ),
            timeout_secs: 60,
        }
    }
}

impl ExternalLiveTransport {
    pub fn detect() -> Result<Self, PrinterError> {
        let t = Self::default();
        which_ok(&t.curl).map_err(|e| {
            PrinterError::Transport(format!(
                "curl not found ({e}); install curl or set CADRION_CURL"
            ))
        })?;
        which_ok(&t.mosquitto_pub).map_err(|e| {
            PrinterError::Transport(format!(
                "mosquitto_pub not found ({e}); install mosquitto-clients or set CADRION_MOSQUITTO_PUB"
            ))
        })?;
        Ok(t)
    }
}

fn which_ok(bin: &Path) -> Result<(), String> {
    if bin.is_absolute() && bin.is_file() {
        return Ok(());
    }
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "command -v {}",
            shell_escape(&bin.display().to_string())
        ))
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} not on PATH", bin.display()))
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl BambuTransport for ExternalLiveTransport {
    fn ftps_upload(
        &self,
        host: &str,
        access_code: &str,
        remote_name: &str,
        bytes: &[u8],
    ) -> Result<(), PrinterError> {
        let tmp = tempfile_path(remote_name)?;
        std::fs::write(&tmp, bytes)
            .map_err(|e| PrinterError::Transport(format!("write temp gcode: {e}")))?;
        let url = format!("ftps://{host}/{remote_name}");
        let out = std::process::Command::new(&self.curl)
            .args([
                "-sS",
                "--connect-timeout",
                "15",
                "--max-time",
                &self.timeout_secs.to_string(),
                "--ftp-pasv",
                "--ssl-reqd",
                "-k", // Bambu LAN cert is self-signed
                "-u",
                &format!("bblp:{access_code}"),
                "-T",
                &tmp.display().to_string(),
                &url,
            ])
            .output()
            .map_err(|e| PrinterError::Transport(format!("spawn curl: {e}")))?;
        let _ = std::fs::remove_file(&tmp);
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            return Err(PrinterError::Transport(format!(
                "curl FTPS upload failed (status {:?}): {stderr}{stdout}",
                out.status.code()
            )));
        }
        Ok(())
    }

    fn mqtt_start_print(
        &self,
        host: &str,
        serial: &str,
        access_code: &str,
        remote_name: &str,
    ) -> Result<(), PrinterError> {
        let topic = format!("device/{serial}/request");
        // Community LAN payload (Bambu Studio-adjacent). Filename as uploaded via FTPS.
        let payload = serde_json::json!({
            "print": {
                "sequence_id": "0",
                "command": "gcode_file",
                "param": format!("/{remote_name}"),
            }
        });
        let payload_s = payload.to_string();
        let out = std::process::Command::new(&self.mosquitto_pub)
            .args([
                "-h",
                host,
                "-p",
                "8883",
                "--insecure",
                "-u",
                "bblp",
                "-P",
                access_code,
                "-t",
                &topic,
                "-m",
                &payload_s,
                "-q",
                "0",
            ])
            .output()
            .map_err(|e| PrinterError::Transport(format!("spawn mosquitto_pub: {e}")))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(PrinterError::Transport(format!(
                "mosquitto_pub failed (status {:?}): {stderr}",
                out.status.code()
            )));
        }
        Ok(())
    }
}

fn tempfile_path(remote_name: &str) -> Result<PathBuf, PrinterError> {
    let safe: String = remote_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let dir = std::env::temp_dir();
    Ok(dir.join(format!("cadrion-bambu-{safe}")))
}

pub trait Printer: Send + Sync {
    fn info(&self) -> &PrinterInfo;
    fn status(&self) -> Result<serde_json::Value, PrinterError>;
    fn dry_run(
        &self,
        gcode_path: &Path,
        volume: &PrinterVolume,
    ) -> Result<DryRunReport, PrinterError>;
    fn start(
        &self,
        req: &StartRequest,
        allowlist: &BTreeSet<String>,
    ) -> Result<StartGate, PrinterError>;
}

/// Bambu Lab LAN adapter.
///
/// **Safety:** `start` never touches the network unless `req.live == true` **and**
/// all gates pass (allowlist + sha256 + confirm=START + gcode-check) **and**
/// access code + serial are configured.
pub struct BambuAdapter {
    info: PrinterInfo,
    access_code: Option<String>,
    transport: Arc<dyn BambuTransport>,
}

impl std::fmt::Debug for BambuAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BambuAdapter")
            .field("info", &self.info)
            .field("has_access_code", &self.access_code.is_some())
            .finish()
    }
}

impl BambuAdapter {
    pub fn new(id: impl Into<String>, host: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            info: PrinterInfo {
                id: id.into(),
                model: model.into(),
                host: host.into(),
                allowlisted: false,
                serial: None,
            },
            access_code: None,
            transport: Arc::new(NullTransport),
        }
    }

    pub fn with_allowlisted(mut self, yes: bool) -> Self {
        self.info.allowlisted = yes;
        self
    }

    pub fn with_serial(mut self, serial: impl Into<String>) -> Self {
        self.info.serial = Some(serial.into());
        self
    }

    pub fn with_access_code(mut self, code: impl Into<String>) -> Self {
        self.access_code = Some(code.into());
        self
    }

    pub fn with_transport(mut self, t: Arc<dyn BambuTransport>) -> Self {
        self.transport = t;
        self
    }

    /// Build from CLI/env: access code from arg or `CADRION_BAMBU_ACCESS_CODE`.
    pub fn from_env(
        id: impl Into<String>,
        host: impl Into<String>,
        model: impl Into<String>,
        serial: Option<String>,
        access_code: Option<String>,
    ) -> Self {
        let code =
            access_code.or_else(|| cadrion_kernel::env_var("CADRION_BAMBU_ACCESS_CODE").ok());
        let serial = serial.or_else(|| cadrion_kernel::env_var("CADRION_BAMBU_SERIAL").ok());
        let mut a = Self::new(id, host, model);
        if let Some(s) = serial {
            a = a.with_serial(s);
        }
        if let Some(c) = code {
            a = a.with_access_code(c);
        }
        a
    }
}

impl Printer for BambuAdapter {
    fn info(&self) -> &PrinterInfo {
        &self.info
    }

    fn status(&self) -> Result<serde_json::Value, PrinterError> {
        Ok(serde_json::json!({
            "ok": true,
            "printer": self.info,
            "mode": if self.access_code.is_some() { "credentials_present" } else { "no_credentials" },
            "state": "unknown_local",
            "note": "Status is local metadata only; live MQTT status poll is not implemented yet.",
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
            "dry-run only: no FTPS upload performed".into(),
            format!("target ftps://{}/ (not contacted)", self.info.host),
            format!(
                "to live-start later: --sha256 {sha} --confirm {CONFIRM_START} --allowlist {} --live",
                self.info.id
            ),
        ];
        if self.access_code.is_none() {
            notes.push("no access code set (CADRION_BAMBU_ACCESS_CODE or --access-code)".into());
        }
        if self.info.serial.is_none() {
            notes.push(
                "no serial set (CADRION_BAMBU_SERIAL or --serial) — required for MQTT".into(),
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
            would_upload_to: format!("ftps://{}/", self.info.host),
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

        // Gates passed.
        if !req.live {
            return Ok(StartGate {
                ok: false,
                errors: vec![
                    "gates passed, but live I/O not requested — re-run with --live to contact printer"
                        .into(),
                ],
                live_attempted: false,
                uploaded_as: None,
                mqtt_topic: None,
                notes: Some(vec![
                    "safety: default start is dry (no network) even when gates pass".into(),
                    format!("hash ok: {sha}"),
                ]),
            });
        }

        let access = match &self.access_code {
            Some(c) if !c.is_empty() => c.clone(),
            _ => {
                return Ok(StartGate {
                    ok: false,
                    errors: vec![
                        "live start needs access code (--access-code or CADRION_BAMBU_ACCESS_CODE)"
                            .into(),
                    ],
                    live_attempted: false,
                    uploaded_as: None,
                    mqtt_topic: None,
                    notes: None,
                });
            }
        };
        let serial = match &self.info.serial {
            Some(s) if !s.is_empty() => s.clone(),
            _ => {
                return Ok(StartGate {
                    ok: false,
                    errors: vec![
                        "live start needs printer serial (--serial or CADRION_BAMBU_SERIAL)".into(),
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

        let mut notes = vec!["gates passed; attempting live FTPS + MQTT".into()];
        if let Err(e) = self
            .transport
            .ftps_upload(&self.info.host, &access, &remote, &bytes)
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
        notes.push(format!(
            "FTPS upload ok → ftps://{}/{remote}",
            self.info.host
        ));
        let topic = format!("device/{serial}/request");
        if let Err(e) = self
            .transport
            .mqtt_start_print(&self.info.host, &serial, &access, &remote)
        {
            return Ok(StartGate {
                ok: false,
                errors: vec![format!("FTPS ok but MQTT start failed: {e}")],
                live_attempted: true,
                uploaded_as: Some(remote),
                mqtt_topic: Some(topic),
                notes: Some(notes),
            });
        }
        notes.push(format!("MQTT print command published on {topic}"));
        Ok(StartGate {
            ok: true,
            errors: vec![],
            live_attempted: true,
            uploaded_as: Some(remote),
            mqtt_topic: Some(topic),
            notes: Some(notes),
        })
    }
}

pub fn hex_sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Evaluate start gates without a live adapter (unit-test helper).
pub fn evaluate_start_gates(
    req: &StartRequest,
    allowlist: &BTreeSet<String>,
    file_sha: &str,
    gcode_ok: bool,
) -> StartGate {
    let mut errors = Vec::new();
    if req.confirm != CONFIRM_START {
        errors.push("confirm must be START".into());
    }
    if !allowlist.contains(&req.printer_id) {
        errors.push("printer not allowlisted".into());
    }
    if !file_sha.eq_ignore_ascii_case(&req.gcode_sha256) {
        errors.push("hash mismatch".into());
    }
    if !gcode_ok {
        errors.push("gcode-check failed".into());
    }
    StartGate {
        ok: errors.is_empty(),
        errors,
        live_attempted: false,
        uploaded_as: None,
        mqtt_topic: None,
        notes: None,
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
        let p = BambuAdapter::new("bambu:x1c-01", "192.168.1.50", "X1C");
        let r = p.dry_run(f.path(), &PrinterVolume::default()).unwrap();
        assert_eq!(r.gcode_sha256.len(), 64);
        assert!(r.ok);
    }

    #[test]
    fn start_requires_confirm() {
        let mut allow = BTreeSet::new();
        allow.insert("bambu:x1c-01".into());
        let gate = evaluate_start_gates(
            &StartRequest {
                printer_id: "bambu:x1c-01".into(),
                gcode_path: "x.gcode".into(),
                gcode_sha256: "abc".into(),
                confirm: "yes".into(),
                live: false,
                remote_name: None,
            },
            &allow,
            "abc",
            true,
        );
        assert!(!gate.ok);
        assert!(gate.errors.iter().any(|e| e.contains("confirm")));
    }

    #[test]
    fn start_gates_pass_when_complete() {
        let mut allow = BTreeSet::new();
        allow.insert("bambu:x1c-01".into());
        let gate = evaluate_start_gates(
            &StartRequest {
                printer_id: "bambu:x1c-01".into(),
                gcode_path: "x.gcode".into(),
                gcode_sha256: "abc".into(),
                confirm: CONFIRM_START.into(),
                live: false,
                remote_name: None,
            },
            &allow,
            "abc",
            true,
        );
        assert!(gate.ok);
    }

    #[test]
    fn start_without_live_refuses_network_even_if_gates_ok() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let bytes = std::fs::read(f.path()).unwrap();
        let sha = hex_sha256(&bytes);
        let rec = RecordingTransport::new();
        let p = BambuAdapter::new("bambu:x1c-01", "192.168.1.50", "X1C")
            .with_serial("01P00A000000000")
            .with_access_code("12345678")
            .with_transport(Arc::new(rec.clone()));
        let mut allow = BTreeSet::new();
        allow.insert("bambu:x1c-01".into());
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "bambu:x1c-01".into(),
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
        assert!(gate.errors.iter().any(|e| e.contains("--live")));
    }

    #[test]
    fn start_live_calls_transport_when_gates_pass() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let bytes = std::fs::read(f.path()).unwrap();
        let sha = hex_sha256(&bytes);
        let rec = RecordingTransport::new();
        let p = BambuAdapter::new("bambu:x1c-01", "192.168.1.50", "X1C")
            .with_serial("01P00A000000000")
            .with_access_code("12345678")
            .with_transport(Arc::new(rec.clone()));
        let mut allow = BTreeSet::new();
        allow.insert("bambu:x1c-01".into());
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "bambu:x1c-01".into(),
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
        assert_eq!(gate.uploaded_as.as_deref(), Some("job.gcode"));
        let log = rec.entries();
        assert!(log.iter().any(|e| e.starts_with("ftps_upload")));
        assert!(log.iter().any(|e| e.starts_with("mqtt_start")));
    }

    #[test]
    fn live_without_allowlist_never_touches_transport() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "G28\nG1 X1 Y1 Z0.2\n").unwrap();
        let sha = hex_sha256(&std::fs::read(f.path()).unwrap());
        let rec = RecordingTransport::new();
        let p = BambuAdapter::new("bambu:x1c-01", "192.168.1.50", "X1C")
            .with_serial("S")
            .with_access_code("C")
            .with_transport(Arc::new(rec.clone()));
        let allow = BTreeSet::new(); // empty
        let gate = p
            .start(
                &StartRequest {
                    printer_id: "bambu:x1c-01".into(),
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
