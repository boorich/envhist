use crate::daemon_client;
use anyhow::Result;
use chrono::Utc;
use envhist_core::{session::Session, storage::Snapshot, storage::Storage};
use std::process;

fn current_session() -> Option<Session> {
    daemon_client::get_session(process::id()).ok().flatten()
}

pub fn snapshot(name: Option<String>, description: Option<String>) -> Result<()> {
    let storage = Storage::new()?;
    let current_env = Storage::get_current_env();

    let snapshot_name =
        name.unwrap_or_else(|| format!("snapshot-{}", Utc::now().format("%Y%m%d-%H%M%S")));

    let session = current_session();

    let snapshot = Snapshot {
        name: snapshot_name.clone(),
        created_at: Utc::now(),
        description,
        environment: current_env,
        tags: Vec::new(),
        session_id: session.as_ref().map(|s| s.id),
    };

    storage.save_snapshot(&snapshot, session.as_ref())?;
    println!("✓ Saved snapshot: {}", snapshot_name);

    Ok(())
}

pub fn list() -> Result<()> {
    let storage = Storage::new()?;
    let session = current_session();
    let snapshots = storage.list_snapshots(session.as_ref())?;

    if snapshots.is_empty() {
        println!("No snapshots found.");
        return Ok(());
    }

    println!("Snapshots:");
    for snap in snapshots {
        let session_info = if let Some(sid) = snap.session_id {
            format!(" (session: {})", sid)
        } else {
            String::new()
        };

        let desc = snap
            .description
            .as_ref()
            .map(|d| format!(" - {}", d))
            .unwrap_or_default();

        println!(
            "  {} - {}{}{}",
            snap.name,
            snap.created_at.format("%Y-%m-%d %H:%M:%S"),
            session_info,
            desc
        );
    }

    Ok(())
}

pub fn restore(name: String, dry_run: bool) -> Result<()> {
    let storage = Storage::new()?;
    let session = current_session();
    let snapshot = storage.load_snapshot(&name, session.as_ref())?;

    if dry_run {
        println!("Would restore snapshot: {}", name);
        println!("Environment variables:");
        for (key, value) in snapshot.environment.iter() {
            println!("  {}={}", key, value);
        }
        return Ok(());
    }

    // Restore each variable
    for (key, value) in snapshot.environment.iter() {
        std::env::set_var(key, value);
        println!("export {}=\"{}\"", key, value.replace("\"", "\\\""));
    }

    println!("✓ Restored snapshot: {}", name);
    println!("\nNote: Run the export commands above in your shell to apply changes.");

    Ok(())
}

pub fn delete(name: String) -> Result<()> {
    let storage = Storage::new()?;
    let session = current_session();
    storage.delete_snapshot(&name, session.as_ref())?;
    println!("✓ Deleted snapshot: {}", name);

    Ok(())
}
