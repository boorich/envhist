use crate::daemon_client;
use anyhow::Result;
use chrono::{DateTime, Utc};
use envhist_core::{session::Session, storage::Storage, storage::TimelineEntry};
use std::process;

pub fn log(since: Option<String>, grep: Option<String>) -> Result<()> {
    let storage = Storage::new()?;
    let pid = process::id();

    // Try to get session for this PID
    let session = get_session_for_pid(pid)?;
    let entries = storage.read_timeline(&session)?;

    let filtered_entries: Vec<&TimelineEntry> = entries
        .iter()
        .filter(|entry| {
            // Filter by since
            if let Some(ref since_str) = since {
                if !matches_since(entry.timestamp, since_str) {
                    return false;
                }
            }

            // Filter by grep
            if let Some(ref pattern) = grep {
                if !entry.key.contains(pattern) {
                    return false;
                }
            }

            true
        })
        .collect();

    if filtered_entries.is_empty() {
        println!("No timeline entries found.");
        return Ok(());
    }

    for entry in filtered_entries {
        let action_str = match entry.action {
            envhist_core::storage::Action::Set => "SET",
            envhist_core::storage::Action::Unset => "UNSET",
        };

        let value_str = if let Some(ref v) = entry.value {
            format!(" = {}", v)
        } else {
            String::new()
        };

        println!(
            "[{}] {} {} {}{}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            action_str,
            entry.key,
            value_str,
            if let Some(ref prev) = entry.prev {
                format!(" (was: {})", prev)
            } else {
                String::new()
            }
        );
    }

    Ok(())
}

pub fn show(var_name: String) -> Result<()> {
    let storage = Storage::new()?;
    let pid = process::id();

    let session = get_session_for_pid(pid)?;
    let entries = storage.read_timeline(&session)?;

    let var_entries: Vec<&TimelineEntry> = entries.iter().filter(|e| e.key == var_name).collect();

    if var_entries.is_empty() {
        println!("No history found for variable: {}", var_name);
        return Ok(());
    }

    println!("History for {}:", var_name);
    for entry in var_entries {
        let action_str = match entry.action {
            envhist_core::storage::Action::Set => "SET",
            envhist_core::storage::Action::Unset => "UNSET",
        };

        let value_str = if let Some(ref v) = entry.value {
            format!(" = {}", v)
        } else {
            String::new()
        };

        println!(
            "  [{}] {} {}{}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
            action_str,
            value_str,
            if let Some(ref prev) = entry.prev {
                format!(" (was: {})", prev)
            } else {
                String::new()
            }
        );
    }

    Ok(())
}

fn get_session_for_pid(pid: u32) -> Result<Session> {
    if let Ok(Some(session)) = daemon_client::get_session(pid) {
        return Ok(session);
    }

    // Fallback: try to find session metadata manually
    let sessions_dir = envhist_core::Config::sessions_dir();
    if sessions_dir.exists() {
        for entry in std::fs::read_dir(&sessions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let metadata_path = path.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(metadata) = Session::load_metadata(&metadata_path) {
                        if metadata.session.pid == pid {
                            return Ok(metadata.session);
                        }
                    }
                }
            }
        }
    }

    // Create temporary session if nothing else worked
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string());
    Ok(Session::new(pid, shell))
}

fn matches_since(timestamp: DateTime<Utc>, since: &str) -> bool {
    // Simple parsing for "1 hour ago", "2 days ago", etc.
    // For MVP, just check if timestamp is recent
    let now = Utc::now();
    let duration = now - timestamp;

    if since.contains("hour") {
        if let Some(hours) = parse_number(since) {
            return duration.num_hours() < hours as i64;
        }
    } else if since.contains("day") {
        if let Some(days) = parse_number(since) {
            return duration.num_days() < days as i64;
        }
    }

    // Default: show last 24 hours
    duration.num_hours() < 24
}

fn parse_number(s: &str) -> Option<u32> {
    s.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}
