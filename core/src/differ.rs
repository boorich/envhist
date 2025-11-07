use crate::Env;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvDiff {
    Added {
        key: String,
        value: String,
    },
    Removed {
        key: String,
        old_value: String,
    },
    Changed {
        key: String,
        old_value: String,
        new_value: String,
    },
    Unchanged {
        key: String,
        value: String,
    },
}

pub fn diff_envs(old: &Env, new: &Env) -> Vec<EnvDiff> {
    let mut diffs = Vec::new();

    // Find added and changed
    for (key, new_val) in new.iter() {
        match old.get(key) {
            None => diffs.push(EnvDiff::Added {
                key: key.clone(),
                value: new_val.clone(),
            }),
            Some(old_val) if old_val != new_val => {
                diffs.push(EnvDiff::Changed {
                    key: key.clone(),
                    old_value: old_val.clone(),
                    new_value: new_val.clone(),
                });
            }
            Some(val) => {
                diffs.push(EnvDiff::Unchanged {
                    key: key.clone(),
                    value: val.clone(),
                });
            }
        }
    }

    // Find removed
    for (key, old_val) in old.iter() {
        if !new.contains_key(key) {
            diffs.push(EnvDiff::Removed {
                key: key.clone(),
                old_value: old_val.clone(),
            });
        }
    }

    // Sort by key for consistent output
    diffs.sort_by(|a, b| {
        let key_a = match a {
            EnvDiff::Added { key, .. }
            | EnvDiff::Removed { key, .. }
            | EnvDiff::Changed { key, .. }
            | EnvDiff::Unchanged { key, .. } => key,
        };
        let key_b = match b {
            EnvDiff::Added { key, .. }
            | EnvDiff::Removed { key, .. }
            | EnvDiff::Changed { key, .. }
            | EnvDiff::Unchanged { key, .. } => key,
        };
        key_a.cmp(key_b)
    });

    diffs
}

pub fn format_diff(diffs: &[EnvDiff], show_unchanged: bool) -> String {
    let mut output = String::new();

    let mut added_count = 0;
    let mut removed_count = 0;
    let mut changed_count = 0;

    for diff in diffs {
        match diff {
            EnvDiff::Added { key, value } => {
                output.push_str(&format!("+ {}: {}\n", key, value));
                added_count += 1;
            }
            EnvDiff::Removed { key, old_value } => {
                output.push_str(&format!("- {}: {}\n", key, old_value));
                removed_count += 1;
            }
            EnvDiff::Changed {
                key,
                old_value,
                new_value,
            } => {
                output.push_str(&format!("~ {}:\n", key));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_envs() {
        let mut old = Env::new();
        old.insert("VAR1".to_string(), "value1".to_string());
        old.insert("VAR2".to_string(), "value2".to_string());

        let mut new = Env::new();
        new.insert("VAR1".to_string(), "value1".to_string()); // unchanged
        new.insert("VAR2".to_string(), "value2_modified".to_string()); // changed
        new.insert("VAR3".to_string(), "value3".to_string()); // added

        let diffs = diff_envs(&old, &new);

        assert_eq!(diffs.len(), 4); // 1 unchanged, 1 changed, 1 added, 1 removed (VAR2 from old)
    }
}
