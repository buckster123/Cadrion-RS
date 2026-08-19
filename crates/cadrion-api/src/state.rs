//! Shared API state.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use crate::jobs::{Job, JobEvent};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind: String,
    /// If set, require `Authorization: Bearer <token>`.
    pub token: Option<String>,
    pub project_root: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:7410".into(),
            token: None,
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub cfg: AppConfig,
    pub jobs: Arc<RwLock<HashMap<String, Job>>>,
    pub events: broadcast::Sender<JobEvent>,
}

impl AppState {
    pub fn new(cfg: AppConfig) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            cfg,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            events,
        }
    }
}
