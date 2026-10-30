use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "tomet";
const APPLICATION: &str = "ordiola";
const CONFIG_FILE_NAME: &str = "config.json";

/// Entry representing a previously opened vault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultEntry {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: u64,
}

impl VaultEntry {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("vault")
            .to_string();
        let last_opened = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            path,
            name,
            last_opened,
        }
    }
}

/// Global Ordiola application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    pub recent_vaults: Vec<VaultEntry>,
    pub last_vault: Option<PathBuf>,
}

impl AppConfig {
    /// Load configuration from user config directory, or fallback to default if not present.
    pub fn load() -> Self {
        Self::load_from_path(Self::config_file_path())
    }

    /// Save configuration to user config directory.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = Self::config_file_path() {
            self.save_to_path(&path)?;
        }
        Ok(())
    }

    /// Load configuration from a specific file path.
    pub fn load_from_path(path_opt: Option<PathBuf>) -> Self {
        let Some(path) = path_opt else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save configuration to a specific file path.
    pub fn save_to_path(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add or update a vault in recent vaults list, moving it to the front.
    pub fn add_recent_vault(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

        // Remove existing entry if present
        self.recent_vaults.retain(|v| v.path != canonical);

        // Prepend new entry
        let mut entry = VaultEntry::new(canonical.clone());
        entry.last_opened = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.recent_vaults.insert(0, entry);

        // Keep maximum of 20 recent vaults
        if self.recent_vaults.len() > 20 {
            self.recent_vaults.truncate(20);
        }

        self.last_vault = Some(canonical);
    }

    /// Get standard configuration file location.
    pub fn config_file_path() -> Option<PathBuf> {
        ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
            .map(|proj| proj.config_dir().join(CONFIG_FILE_NAME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_recent_vault() {
        let mut config = AppConfig::default();
        let path1 = PathBuf::from("/tmp/vault1");
        let path2 = PathBuf::from("/tmp/vault2");

        config.add_recent_vault(&path1);
        config.add_recent_vault(&path2);

        assert_eq!(config.recent_vaults.len(), 2);
        assert_eq!(config.recent_vaults[0].path, path2);
        assert_eq!(config.last_vault, Some(path2.clone()));

        // Re-adding path1 moves it to front
        config.add_recent_vault(&path1);
        assert_eq!(config.recent_vaults[0].path, path1);
        assert_eq!(config.recent_vaults[1].path, path2);
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut config = AppConfig::default();
        config.add_recent_vault("/tmp/my-vault");
        config.save_to_path(&config_path).unwrap();

        let loaded = AppConfig::load_from_path(Some(config_path));
        assert_eq!(loaded.recent_vaults.len(), 1);
        assert_eq!(loaded.recent_vaults[0].name, "my-vault");
    }
}
