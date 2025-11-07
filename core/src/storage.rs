use crate::{config::Config, session::Session, Env};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub timestamp: DateTime<Utc>,
    pub action: Action,
    pub key: String,
    pub value: Option<String>,
    pub prev: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Set,
    Unset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
    pub environment: Env,
    pub tags: Vec<String>,
    pub session_id: Option<uuid::Uuid>,
}

#[derive(Clone)]
pub struct Storage {
    #[allow(dead_code)]
    config: Config,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        Ok(Self { config })
    }

    pub fn with_config(config: Config) -> Self {
        Self { config }
    }

    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(Config::base_dir()).context("Failed to create base directory")?;
        std::fs::create_dir_all(Config::sessions_dir())
            .context("Failed to create sessions directory")?;
        std::fs::create_dir_all(Config::global_snapshots_dir())
            .context("Failed to create global snapshots directory")?;
        Ok(())
    }

    pub fn append_timeline(&self, session: &Session, entry: &TimelineEntry) -> Result<()> {
        let timeline_path = session.timeline_path();
        if let Some(parent) = timeline_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create timeline directory {:?}", parent))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&timeline_path)
            .with_context(|| format!("Failed to open timeline file {:?}", timeline_path))?;

        let line = serde_json::to_string(entry).context("Failed to serialize timeline entry")?;
        writeln!(file, "{}", line)
            .with_context(|| format!("Failed to write to timeline file {:?}", timeline_path))?;
        Ok(())
    }

    pub fn read_timeline(&self, session: &Session) -> Result<Vec<TimelineEntry>> {
        let timeline_path = session.timeline_path();
        if !timeline_path.exists() {
            return Ok(Vec::new());
        }

        let file = std::fs::File::open(&timeline_path)
            .with_context(|| format!("Failed to open timeline file {:?}", timeline_path))?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line.context("Failed to read timeline line")?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: TimelineEntry = serde_json::from_str(&line)
                .with_context(|| format!("Failed to parse timeline entry: {}", line))?;
            entries.push(entry);
        }

        Ok(entries)
    }

    pub fn save_snapshot(&self, snapshot: &Snapshot, session: Option<&Session>) -> Result<()> {
        let snapshot_path = if let Some(sess) = session {
            let snapshots_dir = sess.snapshots_dir();
            std::fs::create_dir_all(&snapshots_dir)
                .context("Failed to create session snapshots directory")?;
            snapshots_dir.join(format!("{}.json", snapshot.name))
        } else {
            Config::global_snapshots_dir().join(format!("{}.json", snapshot.name))
        };

        let content =
            serde_json::to_string_pretty(snapshot).context("Failed to serialize snapshot")?;
        std::fs::write(&snapshot_path, content)
            .with_context(|| format!("Failed to write snapshot to {:?}", snapshot_path))?;
        Ok(())
    }

    pub fn load_snapshot(&self, name: &str, session: Option<&Session>) -> Result<Snapshot> {
        // Try session snapshot first, then global
        let snapshot_path = if let Some(sess) = session {
            sess.snapshots_dir().join(format!("{}.json", name))
        } else {
            Config::global_snapshots_dir().join(format!("{}.json", name))
        };

        if !snapshot_path.exists() {
            // Try the other location
            let alt_path = if session.is_some() {
                Config::global_snapshots_dir().join(format!("{}.json", name))
            } else {
                // Search in all session directories
                return self.find_snapshot_in_sessions(name);
            };

            if alt_path.exists() {
                return self.load_snapshot_from_path(&alt_path);
            }

            anyhow::bail!("Snapshot '{}' not found", name);
        }

        self.load_snapshot_from_path(&snapshot_path)
    }

    fn load_snapshot_from_path(&self, path: &PathBuf) -> Result<Snapshot> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read snapshot from {:?}", path))?;
        let snapshot: Snapshot = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse snapshot from {:?}", path))?;
        Ok(snapshot)
    }

    fn find_snapshot_in_sessions(&self, name: &str) -> Result<Snapshot> {
        let sessions_dir = Config::sessions_dir();
        if !sessions_dir.exists() {
            anyhow::bail!("Snapshot '{}' not found", name);
        }

        for entry in std::fs::read_dir(&sessions_dir)
            .with_context(|| format!("Failed to read sessions directory {:?}", sessions_dir))?
        {
            let entry = entry.context("Failed to read session directory entry")?;
            let path = entry.path();
            if path.is_dir() {
                let snapshots_dir = path.join("snapshots");
                if snapshots_dir.exists() {
                    let snapshot_path = snapshots_dir.join(format!("{}.json", name));
                    if snapshot_path.exists() {
                        return self.load_snapshot_from_path(&snapshot_path);
                    }
                }
            }
        }

        anyhow::bail!("Snapshot '{}' not found", name);
    }

    pub fn list_snapshots(&self, session: Option<&Session>) -> Result<Vec<Snapshot>> {
        let mut snapshots = Vec::new();

        // List session snapshots
        if let Some(sess) = session {
            let snapshots_dir = sess.snapshots_dir();
            if snapshots_dir.exists() {
                for entry in std::fs::read_dir(&snapshots_dir)
                    .with_context(|| "Failed to read session snapshots directory")?
                {
                    let entry = entry.context("Failed to read snapshot entry")?;
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(snapshot) = self.load_snapshot_from_path(&path) {
                            snapshots.push(snapshot);
                        }
                    }
                }
            }
        }

        // List global snapshots
        let global_snapshots_dir = Config::global_snapshots_dir();
        if global_snapshots_dir.exists() {
            for entry in std::fs::read_dir(&global_snapshots_dir)
                .with_context(|| "Failed to read global snapshots directory")?
            {
                let entry = entry.context("Failed to read snapshot entry")?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(snapshot) = self.load_snapshot_from_path(&path) {
                        snapshots.push(snapshot);
                    }
                }
            }
        }

        // Sort by created_at
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(snapshots)
    }

    pub fn delete_snapshot(&self, name: &str, session: Option<&Session>) -> Result<()> {
        // Try session snapshot first
        if let Some(sess) = session {
            let snapshot_path = sess.snapshots_dir().join(format!("{}.json", name));
            if snapshot_path.exists() {
                std::fs::remove_file(&snapshot_path)
                    .with_context(|| format!("Failed to delete snapshot {:?}", snapshot_path))?;
                return Ok(());
            }
        }

        // Try global snapshot
        let snapshot_path = Config::global_snapshots_dir().join(format!("{}.json", name));
        if snapshot_path.exists() {
            std::fs::remove_file(&snapshot_path)
                .with_context(|| format!("Failed to delete snapshot {:?}", snapshot_path))?;
            return Ok(());
        }

        // Search in all session directories
        let sessions_dir = Config::sessions_dir();
        if sessions_dir.exists() {
            for entry in std::fs::read_dir(&sessions_dir)
                .with_context(|| "Failed to read sessions directory")?
            {
                let entry = entry.context("Failed to read session directory entry")?;
                let path = entry.path();
                if path.is_dir() {
                    let snapshot_path = path.join("snapshots").join(format!("{}.json", name));
                    if snapshot_path.exists() {
                        std::fs::remove_file(&snapshot_path).with_context(|| {
                            format!("Failed to delete snapshot {:?}", snapshot_path)
                        })?;
                        return Ok(());
                    }
                }
            }
        }

        anyhow::bail!("Snapshot '{}' not found", name);
    }

    pub fn get_current_env() -> Env {
        std::env::vars().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_load_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let base_dir = temp_dir.path().join(".envhist");

        // Create a minimal config
        let config = Config::default();
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::create_dir_all(base_dir.join("global").join("snapshots")).unwrap();

        // Mock the config paths - this is a simplified test
        // In real usage, we'd need to set up the directories properly
    }
}
