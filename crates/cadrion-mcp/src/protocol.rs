//! Minimal MCP / JSON-RPC types + stdio framing.
//!
//! Supports both:
//! - **NDJSON** (newline-delimited JSON) — current official Python `mcp` SDK / Hermes
//! - **Content-Length** framing — LSP-style / older clients
//!
//! Auto-detect on first byte: `{` → NDJSON, else Content-Length headers.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU8, Ordering};

/// Framing mode once detected (or forced).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Framing {
    Unknown = 0,
    Ndjson = 1,
    ContentLength = 2,
}

static FRAMING: AtomicU8 = AtomicU8::new(0);

fn set_framing(f: Framing) {
    FRAMING.store(f as u8, Ordering::Relaxed);
}

fn current_framing() -> Framing {
    match FRAMING.load(Ordering::Relaxed) {
        1 => Framing::Ndjson,
        2 => Framing::ContentLength,
        _ => Framing::Unknown,
    }
}

/// Force framing before run (tests / env). `ndjson` | `content-length` | auto.
pub fn configure_framing_from_env() {
    match cadrion_kernel::env_var("CADRION_MCP_FRAMING")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "ndjson" | "jsonl" | "nl" => set_framing(Framing::Ndjson),
        "content-length" | "cl" | "lsp" => set_framing(Framing::ContentLength),
        _ => {}
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Read one message from stdin (NDJSON or Content-Length).
pub fn read_message(stdin: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    match current_framing() {
        Framing::Ndjson => read_ndjson(stdin),
        Framing::ContentLength => read_content_length(stdin),
        Framing::Unknown => {
            // peek first non-ws byte
            let mut first = [0u8; 1];
            loop {
                let n = stdin.read(&mut first)?;
                if n == 0 {
                    return Ok(None);
                }
                if first[0] == b' ' || first[0] == b'\t' || first[0] == b'\r' || first[0] == b'\n' {
                    continue;
                }
                break;
            }
            if first[0] == b'{' {
                set_framing(Framing::Ndjson);
                let mut line = vec![first[0]];
                let mut buf = [0u8; 1];
                loop {
                    let n = stdin.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    if buf[0] == b'\n' {
                        break;
                    }
                    if buf[0] != b'\r' {
                        line.push(buf[0]);
                    }
                    if line.len() > 16 * 1024 * 1024 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "ndjson line too large",
                        ));
                    }
                }
                Ok(Some(line))
            } else {
                set_framing(Framing::ContentLength);
                // put back first byte into a chain: we already consumed it
                let mut headers = vec![first[0]];
                read_content_length_continue(stdin, &mut headers)
            }
        }
    }
}

fn read_ndjson(stdin: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let mut buf = [0u8; 1];
    let mut saw_byte = false;
    loop {
        let n = stdin.read(&mut buf)?;
        if n == 0 {
            // EOF: empty stream → None; partial line → deliver it
            return if !saw_byte || line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        saw_byte = true;
        if buf[0] == b'\n' {
            if line.is_empty() {
                continue; // skip blank lines
            }
            break;
        }
        if buf[0] != b'\r' {
            line.push(buf[0]);
        }
        if line.len() > 16 * 1024 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "ndjson line too large",
            ));
        }
    }
    Ok(Some(line))
}

fn read_content_length(stdin: &mut impl Read) -> std::io::Result<Option<Vec<u8>>> {
    let mut headers = Vec::new();
    read_content_length_continue(stdin, &mut headers)
}

fn read_content_length_continue(
    stdin: &mut impl Read,
    headers: &mut Vec<u8>,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut buf = [0u8; 1];
    // read until \r\n\r\n or \n\n
    loop {
        let n = stdin.read(&mut buf)?;
        if n == 0 {
            return if headers.is_empty() {
                Ok(None)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "eof mid-headers",
                ))
            };
        }
        headers.push(buf[0]);
        if headers.ends_with(b"\r\n\r\n") || headers.ends_with(b"\n\n") {
            break;
        }
        if headers.len() > 8192 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "headers too large",
            ));
        }
    }
    let header_str = String::from_utf8_lossy(headers);
    let mut content_length = None;
    for line in header_str.lines() {
        let line = line.trim();
        if let Some(v) = line
            .strip_prefix("Content-Length:")
            .or_else(|| line.strip_prefix("content-length:"))
        {
            content_length = v.trim().parse::<usize>().ok();
        }
    }
    let len = content_length.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length")
    })?;
    let mut body = vec![0u8; len];
    stdin.read_exact(&mut body)?;
    Ok(Some(body))
}

pub fn write_message(stdout: &mut impl Write, body: &[u8]) -> std::io::Result<()> {
    match current_framing() {
        Framing::Ndjson | Framing::Unknown => {
            // Prefer NDJSON when unknown (Hermes / official SDK)
            stdout.write_all(body)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
            Ok(())
        }
        Framing::ContentLength => {
            write!(stdout, "Content-Length: {}\r\n\r\n", body.len())?;
            stdout.write_all(body)?;
            stdout.flush()?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::Mutex;

    // FRAMING is process-global; serialize tests that touch it.
    static FRAMING_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn ndjson_roundtrip() {
        let _g = FRAMING_TEST_LOCK.lock().unwrap();
        set_framing(Framing::Unknown);
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
        let mut cur = Cursor::new(&input[..]);
        let body = read_message(&mut cur).unwrap().unwrap();
        assert!(body.starts_with(b"{"));
        assert_eq!(current_framing(), Framing::Ndjson);
        let mut out = Vec::new();
        write_message(&mut out, body.as_slice()).unwrap();
        assert!(out.ends_with(b"\n"));
        set_framing(Framing::Unknown);
    }

    #[test]
    fn content_length_roundtrip() {
        let _g = FRAMING_TEST_LOCK.lock().unwrap();
        set_framing(Framing::Unknown);
        let payload = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let mut msg = format!("Content-Length: {}\r\n\r\n", payload.len()).into_bytes();
        msg.extend_from_slice(payload);
        let mut cur = Cursor::new(msg);
        let body = read_message(&mut cur).unwrap().unwrap();
        assert_eq!(body, payload);
        assert_eq!(current_framing(), Framing::ContentLength);
        set_framing(Framing::Unknown);
    }
}
