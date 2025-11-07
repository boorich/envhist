use anyhow::{Context, Result};
use chrono::Utc;
use envhist_core::{
    session::Session, storage::Action, storage::Storage, storage::TimelineEntry, Config, Env,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::RwLock,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvEvent {
    Set {
        pid: u32,
        key: String,
        value: String,
    },
    Unset {
        pid: u32,
        key: String,
    },
    Capture {
        pid: u32,
        env: Env,
    },
    GetSession {
        pid: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvResponse {
    Ok,
    Session { session: Session },
    Error { message: String },
}

pub struct EnvHistDaemon {
    storage: Storage,
    sessions: Arc<RwLock<HashMap<u32, Session>>>,
    config: Config,
}

impl EnvHistDaemon {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let storage = Storage::with_config(config.clone());
        storage.ensure_directories()?;

        Ok(Self {
            storage,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    pub async fn run(&self, socket_path: std::path::PathBuf) -> Result<()> {
        // Remove old socket if it exists
        if socket_path.exists() {
            std::fs::remove_file(&socket_path).context("Failed to remove existing socket")?;
        }

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("Failed to bind to socket {:?}", socket_path))?;

        eprintln!("Daemon listening on {:?}", socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let sessions = Arc::clone(&self.sessions);
                    let storage = self.storage.clone();
                    let config = self.config.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, sessions, storage, config).await
                        {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }
    }

    async fn handle_client(
        mut stream: UnixStream,
        sessions: Arc<RwLock<HashMap<u32, Session>>>,
        storage: Storage,
        config: Config,
    ) -> Result<()> {
        let (reader, mut writer) = stream.split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        while reader.read_line(&mut line).await? > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }

            let event: EnvEvent = match serde_json::from_str(trimmed) {
                Ok(e) => e,
                Err(e) => {
                    let response = EnvResponse::Error {
                        message: format!("Failed to parse event: {}", e),
                    };
                    let response_json = serde_json::to_string(&response)?;
                    writer.write_all(response_json.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    line.clear();
                    continue;
                }
            };

            let response = Self::handle_event(event, &sessions, &storage, &config).await;
            let response_json = serde_json::to_string(&response)?;
            writer.write_all(response_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            line.clear();
        }

        Ok(())
    }

    async fn handle_event(
        event: EnvEvent,
        sessions: &Arc<RwLock<HashMap<u32, Session>>>,
        storage: &Storage,
        config: &Config,
    ) -> EnvResponse {
        match event {
            EnvEvent::Set { pid, key, value } => {
                if !config.should_track(&key) {
                    return EnvResponse::Ok;
                }

                match Self::get_or_create_session(pid, sessions).await {
                    Ok(session) => {
                        // Get previous value from session metadata if available
                        let prev = Self::get_previous_value(&session, &key, storage).await;

                        let entry = TimelineEntry {
                            timestamp: Utc::now(),
                            action: Action::Set,
                            key: key.clone(),
                            value: Some(value.clone()),
                            prev,
                        };

                        if let Err(e) = storage.append_timeline(&session, &entry) {
                            return EnvResponse::Error {
                                message: format!("Failed to append timeline: {}", e),
                            };
                        }

                        // Update session timestamp
                        {
                            let mut sessions_guard = sessions.write().await;
                            if let Some(sess) = sessions_guard.get_mut(&pid) {
                                sess.update_timestamp();
                            }
                        }

                        EnvResponse::Ok
                    }
                    Err(e) => EnvResponse::Error {
                        message: format!("Failed to get session: {}", e),
                    },
                }
            }
            EnvEvent::Unset { pid, key } => {
                if !config.should_track(&key) {
                    return EnvResponse::Ok;
                }

                match Self::get_or_create_session(pid, sessions).await {
                    Ok(session) => {
                        let prev = Self::get_previous_value(&session, &key, storage).await;

                        let entry = TimelineEntry {
                            timestamp: Utc::now(),
                            action: Action::Unset,
                            key: key.clone(),
                            value: None,
                            prev,
                        };

                        if let Err(e) = storage.append_timeline(&session, &entry) {
                            return EnvResponse::Error {
                                message: format!("Failed to append timeline: {}", e),
                            };
                        }

                        EnvResponse::Ok
                    }
                    Err(e) => EnvResponse::Error {
                        message: format!("Failed to get session: {}", e),
                    },
                }
            }
            EnvEvent::Capture { pid, env } => {
                match Self::get_or_create_session(pid, sessions).await {
                    Ok(session) => {
                        // Save current env state to metadata
                        if let Err(e) = session.save_metadata(&env) {
                            return EnvResponse::Error {
                                message: format!("Failed to save metadata: {}", e),
                            };
                        }
                        EnvResponse::Ok
                    }
                    Err(e) => EnvResponse::Error {
                        message: format!("Failed to get session: {}", e),
                    },
                }
            }
            EnvEvent::GetSession { pid } => {
                match Self::get_or_create_session(pid, sessions).await {
                    Ok(session) => EnvResponse::Session { session },
                    Err(e) => EnvResponse::Error {
                        message: format!("Failed to get session: {}", e),
                    },
                }
            }
        }
    }

    async fn get_or_create_session(
        pid: u32,
        sessions: &Arc<RwLock<HashMap<u32, Session>>>,
    ) -> Result<Session> {
        // Check if session exists
        {
            let sessions_guard = sessions.read().await;
            if let Some(session) = sessions_guard.get(&pid) {
                return Ok(session.clone());
            }
        }

        // Create new session
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
        let session = Session::new(pid, shell);

        {
            let mut sessions_guard = sessions.write().await;
            sessions_guard.insert(pid, session.clone());
        }

        Ok(session)
    }

    async fn get_previous_value(session: &Session, key: &str, storage: &Storage) -> Option<String> {
        // Try to get from metadata first
        if let Ok(metadata) = Session::load_metadata(&session.metadata_path()) {
            return metadata.current_env.get(key).cloned();
        }

        // Try to get from timeline
        if let Ok(entries) = storage.read_timeline(session) {
            for entry in entries.iter().rev() {
                if entry.key == key {
                    return entry.value.clone().or(entry.prev.clone());
                }
            }
        }

        None
    }
}
