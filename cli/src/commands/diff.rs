use anyhow::Result;
use colored::*;
use envhist_core::{
    differ::{diff_envs, EnvDiff},
    storage::Storage,
};

pub fn diff(snapshot1: Option<String>, snapshot2: Option<String>) -> Result<()> {
    let storage = Storage::new()?;

    let (old_env, old_name) = if let Some(ref name) = snapshot1 {
        let snapshot = storage.load_snapshot(name, None)?;
        (snapshot.environment, name.clone())
    } else {
        // Use last snapshot
        let snapshots = storage.list_snapshots(None)?;
        if snapshots.is_empty() {
            anyhow::bail!("No snapshots found. Create one with: envhist snapshot <name>");
        }
        let snapshot = &snapshots[0];
        (snapshot.environment.clone(), snapshot.name.clone())
    };

    let (new_env, new_name) = if let Some(ref name) = snapshot2 {
        let snapshot = storage.load_snapshot(name, None)?;
        (snapshot.environment, name.clone())
    } else {
        // Use current env
        (Storage::get_current_env(), "current".to_string())
    };

    let diffs = diff_envs(&old_env, &new_env);

    println!("--- {} ---", old_name);
    println!("+++ {} +++", new_name);
    println!();

    let output = format_diff_colored(&diffs, false);
    print!("{}", output);

    Ok(())
}

fn format_diff_colored(diffs: &[EnvDiff], show_unchanged: bool) -> String {
    let mut output = String::new();

    let mut added_count = 0;
    let mut removed_count = 0;
    let mut changed_count = 0;

    for diff in diffs {
        match diff {
            EnvDiff::Added { key, value } => {
                output.push_str(&format!("+ {}: {}\n", key.to_string().green(), value));
                added_count += 1;
            }
            EnvDiff::Removed { key, old_value } => {
                output.push_str(&format!("- {}: {}\n", key.to_string().red(), old_value));
                removed_count += 1;
            }
            EnvDiff::Changed {
                key,
                old_value,
                new_value,
            } => {
                output.push_str(&format!("~ {}:\n", key.to_string().yellow()));
                output.push_str(&format!("  - {}\n", old_value));
                output.push_str(&format!("  + {}\n", new_value));
                changed_count += 1;
            }
            EnvDiff::Unchanged { key, value } => {
                if show_unchanged {
                    output.push_str(&format!("  {}: {}\n", key, value));
                }
            }
        }
    }

    if !show_unchanged {
        output.push_str(&format!(
            "\n{} changed, {} added, {} removed\n",
            changed_count, added_count, removed_count
        ));
    }

    output
}
