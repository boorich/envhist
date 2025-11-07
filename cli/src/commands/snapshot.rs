use crate::daemon_client;
use crate::SnapshotArgs;
use anyhow::Result;
use chrono::Utc;
use envhist_core::{storage::Snapshot, storage::Storage};

fn current_session_id() -> Option<uuid::Uuid> {
    daemon_client::get_active_session()
        .ok()
        .flatten()
        .map(|s| s.id)
}

pub fn snapshot(args: SnapshotArgs) -> Result<()> {
    let storage = Storage::new()?;
    let current_env = Storage::get_current_env();

    let snapshot_name = args
        .name
        .unwrap_or_else(|| format!("snapshot-{}", Utc::now().format("%Y%m%d-%H%M%S")));

    let session_id = args.session.then(|| current_session_id()).flatten();

    let snapshot = Snapshot {
        name: snapshot_name.clone(),
        created_at: Utc::now(),
        description: args.description,
        environment: current_env,
        tags: Vec::new(),
        session_id,
    };

    let session = if args.session {
        daemon_client::get_active_session().ok().flatten()
    } else {
        None
    };

    storage.save_snapshot(&snapshot, session.as_ref())?;
    println!("✓ Saved snapshot: {}", snapshot_name);

    Ok(())
}

pub fn list() -> Result<()> {
    let storage = Storage::new()?;
    let session = daemon_client::get_active_session().ok().flatten();
    let mut merged: std::collections::BTreeMap<String, Snapshot> =
        std::collections::BTreeMap::new();

    for snap in storage.list_snapshots(None)? {
        merged.insert(snap.name.clone(), snap);
    }

    if let Some(ref sess) = session {
        for snap in storage.list_snapshots(Some(sess))? {
            merged.insert(snap.name.clone(), snap);
        }
    }

    let mut snapshots: Vec<_> = merged.into_values().collect();
    snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    if snapshots.is_empty() {
        println!("No snapshots found.");
        return Ok(());
    }

    println!("Snapshots:");
    for snap in snapshots {
        let session_info = if let Some(sid) = snap.session_id {
            if Some(sid) == session.as_ref().map(|s| s.id) {
                " (this session)".to_string()
            } else {
                format!(" (session: {})", sid)
            }
        } else {
            " (global)".to_string()
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
    let session = daemon_client::get_active_session().ok().flatten();
    let snapshot = match storage.load_snapshot(&name, None) {
        Ok(global) => global,
        Err(_) => storage.load_snapshot(&name, session.as_ref())?,
    };

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
    let session = daemon_client::get_active_session().ok().flatten();

    if storage.delete_snapshot(&name, None).is_err() {
        storage.delete_snapshot(&name, session.as_ref())?;
    }

    println!("✓ Deleted snapshot: {}", name);

    Ok(())
}
