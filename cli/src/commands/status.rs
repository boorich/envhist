use anyhow::Result;
use envhist_core::{differ::diff_envs, storage::Storage};

pub fn status() -> Result<()> {
    let storage = Storage::new()?;
    let current_env = Storage::get_current_env();

    // Try to get last snapshot
    let snapshots = storage.list_snapshots(None)?;

    if snapshots.is_empty() {
        println!("No snapshots found. Create one with: envhist snapshot <name>");
        return Ok(());
    }

    let last_snapshot = &snapshots[0];
    let snapshot_env = &last_snapshot.environment;

    let diffs = diff_envs(snapshot_env, &current_env);

    let changes: Vec<_> = diffs
        .iter()
        .filter(|d| !matches!(d, envhist_core::differ::EnvDiff::Unchanged { .. }))
        .collect();

    if changes.is_empty() {
        println!("No changes since snapshot: {}", last_snapshot.name);
        return Ok(());
    }

    println!(
        "Changes since snapshot: {} ({})",
        last_snapshot.name,
        last_snapshot.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!();

    for diff in changes {
        match diff {
            envhist_core::differ::EnvDiff::Added { key, value } => {
                println!("+ {}: {}", key, value);
            }
            envhist_core::differ::EnvDiff::Removed { key, old_value } => {
                println!("- {}: {}", key, old_value);
            }
            envhist_core::differ::EnvDiff::Changed {
                key,
                old_value,
                new_value,
            } => {
                println!("~ {}: {} -> {}", key, old_value, new_value);
            }
            _ => {}
        }
    }

    Ok(())
}
