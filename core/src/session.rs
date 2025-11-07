use crate::Env;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub pid: u32,
    pub shell: String,
    pub started_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub session: Session,
    pub current_env: Env,
}

impl Session {
    pub fn new(pid: u32, shell: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            pid,
            shell,
            started_at: now,
            last_updated: now,
        }
    }

    pub fn update_timestamp(&mut self) {
        self.last_updated = Utc::now();
    }

    pub fn session_dir(&self) -> PathBuf {
        crate::config::Config::sessions_dir().join(self.id.to_string())
    }

    pub fn timeline_path(&self) -> PathBuf {
        self.session_dir().join("timeline.jsonl")
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.session_dir().join("metadata.json")
    }

    pub fn snapshots_dir(&self) -> PathBuf {
        self.session_dir().join("snapshots")
    }

    pub fn save_metadata(&self, env: &Env) -> Result<()> {
        let metadata = SessionMetadata {
            session: self.clone(),
            current_env: env.clone(),
        };

        let dir = self.session_dir();
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create session directory {:?}", dir))?;

        let content = serde_json::to_string_pretty(&metadata)
            .context("Failed to serialize session metadata")?;
        std::fs::write(self.metadata_path(), content)
            .with_context(|| "Failed to write session metadata")?;
        Ok(())
    }

    pub fn load_metadata(path: &PathBuf) -> Result<SessionMetadata> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read metadata from {:?}", path))?;
        let metadata: SessionMetadata = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse metadata from {:?}", path))?;
        Ok(metadata)
    }
}
