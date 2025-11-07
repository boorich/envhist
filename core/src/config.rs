use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub filters: FiltersConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_true")]
    pub auto_snapshot: bool,
    #[serde(default = "default_3600")]
    pub auto_snapshot_interval: u64,
    #[serde(default = "default_10000")]
    pub max_timeline_size: usize,
    #[serde(default = "default_true")]
    pub daemon_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiltersConfig {
    #[serde(default = "default_ignore_patterns")]
    pub ignore_patterns: Vec<String>,
    #[serde(default)]
    pub force_track: Vec<String>,
    #[serde(default = "default_ignore_system")]
    pub ignore_system: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_3")]
    pub diff_context: usize,
    #[serde(default = "default_true")]
    pub color: bool,
    #[serde(default = "default_local")]
    pub timezone: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            filters: FiltersConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            auto_snapshot: true,
            auto_snapshot_interval: 3600,
            max_timeline_size: 10000,
            daemon_enabled: true,
        }
    }
}

impl Default for FiltersConfig {
    fn default() -> Self {
        Self {
            ignore_patterns: default_ignore_patterns(),
            force_track: Vec::new(),
            ignore_system: default_ignore_system(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            diff_context: 3,
            color: true,
            timezone: "local".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_3600() -> u64 {
    3600
}

fn default_10000() -> usize {
    10000
}

fn default_3() -> usize {
    3
}

fn default_local() -> String {
    "local".to_string()
}

fn default_ignore_patterns() -> Vec<String> {
    vec![
        ".*PASSWORD.*".to_string(),
        ".*SECRET.*".to_string(),
        ".*TOKEN.*".to_string(),
        "AWS_.*".to_string(),
        "SSH_.*".to_string(),
    ]
}

fn default_ignore_system() -> Vec<String> {
    vec![
        "PATH".to_string(),
        "HOME".to_string(),
        "USER".to_string(),
        "SHELL".to_string(),
        "PWD".to_string(),
        "OLDPWD".to_string(),
        "TERM".to_string(),
        "SHLVL".to_string(),
        "_".to_string(),
        "LS_COLORS".to_string(),
    ]
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if !config_path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config from {:?}", config_path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {:?}", config_path))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory {:?}", parent))?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;
        std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config to {:?}", config_path))?;
        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to find home directory")
            .join(".envhist")
            .join("config.toml")
    }

    pub fn base_dir() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to find home directory")
            .join(".envhist")
    }

    pub fn sessions_dir() -> PathBuf {
        Self::base_dir().join("sessions")
    }

    pub fn global_snapshots_dir() -> PathBuf {
        Self::base_dir().join("global").join("snapshots")
    }

    pub fn daemon_socket_path() -> PathBuf {
        Self::base_dir().join("daemon.sock")
    }

    pub fn should_track(&self, key: &str) -> bool {
        // Check force_track first (highest priority)
        if self.filters.force_track.iter().any(|pattern| {
            Regex::new(pattern)
                .map(|re| re.is_match(key))
                .unwrap_or(false)
        }) {
            return true;
        }

        // Check ignore_system
        if self.filters.ignore_system.contains(&key.to_string()) {
            return false;
        }

        // Check ignore_patterns
        if self.filters.ignore_patterns.iter().any(|pattern| {
            Regex::new(pattern)
                .map(|re| re.is_match(key))
                .unwrap_or(false)
        }) {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_track() {
        let config = Config::default();

        // Should not track system vars
        assert!(!config.should_track("PATH"));
        assert!(!config.should_track("HOME"));

        // Should not track secrets by default
        assert!(!config.should_track("MY_PASSWORD"));
        assert!(!config.should_track("API_SECRET"));

        // Should track normal vars
        assert!(config.should_track("MY_VAR"));
        assert!(config.should_track("CANTON_NODE_1"));

        // Force track should override
        let mut config = Config::default();
        config.filters.force_track.push("MY_PASSWORD".to_string());
        assert!(config.should_track("MY_PASSWORD"));
    }
}
