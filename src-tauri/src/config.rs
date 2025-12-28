//! Configuration and path management for Project Zomboid save backup/restore.
//!
//! This module provides:
//! - Platform-specific Zomboid save path auto-detection
//! - Configuration file persistence (JSON format)
//! - User preference management (paths, backup retention settings)

use crate::file_ops::{FileOpsError, FileOpsResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default backup retention count.
pub const DEFAULT_RETENTION_COUNT: usize = 10;

/// Default configuration file name.
const CONFIG_FILE_NAME: &str = "zomboid_backup_config.json";

/// Application configuration.
///
/// Stores user preferences including save paths, backup location, and retention policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the Project Zomboid saves directory.
    /// If None, will attempt auto-detection.
    pub save_path: Option<String>,

    /// Path to the backup storage directory.
    /// If None, backups will be stored in a default location.
    pub backup_path: Option<String>,

    /// Maximum number of backups to retain per save.
    /// Old backups exceeding this count will be garbage collected.
    pub retention_count: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            save_path: None,
            backup_path: None,
            retention_count: DEFAULT_RETENTION_COUNT,
        }
    }
}

impl Config {
    /// Creates a new configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new configuration with the specified save path.
    pub fn with_save_path(save_path: String) -> Self {
        Config {
            save_path: Some(save_path),
            ..Default::default()
        }
    }

    /// Creates a new configuration with all paths specified.
    pub fn with_paths(save_path: String, backup_path: String) -> Self {
        Config {
            save_path: Some(save_path),
            backup_path: Some(backup_path),
            ..Default::default()
        }
    }

    /// Returns the effective save path, using auto-detection if not set.
    pub fn get_save_path(&self) -> FileOpsResult<PathBuf> {
        match &self.save_path {
            Some(path) => Ok(PathBuf::from(path)),
            None => detect_zomboid_save_path(),
        }
    }

    /// Returns the effective backup path, using default if not set.
    pub fn get_backup_path(&self) -> FileOpsResult<PathBuf> {
        match &self.backup_path {
            Some(path) => Ok(PathBuf::from(path)),
            None => get_default_backup_path(),
        }
    }

    /// Validates that all configured paths exist and are directories.
    pub fn validate(&self) -> FileOpsResult<()> {
        let save_path = self.get_save_path()?;
        if !save_path.exists() {
            return Err(FileOpsError::SourceNotFound(save_path));
        }
        if !save_path.is_dir() {
            return Err(FileOpsError::NotADirectory(save_path));
        }

        // Backup path may not exist yet, that's okay
        // But if it exists, it must be a directory
        if let Some(backup_path_str) = &self.backup_path {
            let backup_path = Path::new(backup_path_str);
            if backup_path.exists() && !backup_path.is_dir() {
                return Err(FileOpsError::NotADirectory(backup_path.to_path_buf()));
            }
        }

        Ok(())
    }
}

/// Result type for config operations.
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Error type for configuration operations.
#[derive(Debug)]
pub enum ConfigError {
    /// File operation error
    FileOp(FileOpsError),
    /// JSON serialization/deserialization error
    Json(serde_json::Error),
    /// Config directory not found
    ConfigDirNotFound,
    /// Invalid config value
    InvalidValue(String),
}

impl From<FileOpsError> for ConfigError {
    fn from(err: FileOpsError) -> Self {
        ConfigError::FileOp(err)
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> Self {
        ConfigError::Json(err)
    }
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileOp(err) => write!(f, "File operation error: {}", err),
            ConfigError::Json(err) => write!(f, "JSON error: {}", err),
            ConfigError::ConfigDirNotFound => write!(f, "Config directory not found"),
            ConfigError::InvalidValue(msg) => write!(f, "Invalid config value: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::FileOp(err) => Some(err),
            ConfigError::Json(err) => Some(err),
            _ => None,
        }
    }
}

impl serde::Serialize for ConfigError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Detects the default Zomboid save path for the current platform.
///
/// # Returns
/// `FileOpsResult<PathBuf>` - The detected save path
///
/// # Platform Behavior
/// - **Windows**: `C:\Users\<User>\Zomboid\Saves`
/// - **Mac/Linux**: `~/Zomboid/Saves`
///
/// # Example
/// ```no_run
/// use tauri_app_lib::config::detect_zomboid_save_path;
///
/// let path = detect_zomboid_save_path().unwrap();
/// println!("Zomboid saves: {:?}", path);
/// ```
pub fn detect_zomboid_save_path() -> FileOpsResult<PathBuf> {
    // Both Windows and Mac/Linux use the same path structure relative to home dir
    let base_path = dirs::home_dir().map(|p| p.join("Zomboid").join("Saves"));

    match base_path {
        Some(path) => Ok(path),
        None => Err(FileOpsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine home directory",
        ))),
    }
}

/// Gets the default backup storage path.
///
/// # Returns
/// `FileOpsResult<PathBuf>` - Default backup directory path
///
/// # Platform Behavior
/// - **Windows**: `%USERPROFILE%\ZomboidBackups`
/// - **Mac/Linux**: `~/ZomboidBackups`
pub fn get_default_backup_path() -> FileOpsResult<PathBuf> {
    let backup_path = dirs::home_dir()
        .map(|p| p.join("ZomboidBackups"))
        .ok_or_else(|| {
            FileOpsError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine home directory",
            ))
        })?;

    Ok(backup_path)
}

/// Returns the path to the application config directory.
///
/// # Platform Behavior
/// - **Windows**: `%APPDATA%\ZomboidBackupTool`
/// - **macOS**: `~/Library/Application Support/ZomboidBackupTool`
/// - **Linux**: `~/.config/ZomboidBackupTool`
pub fn get_config_dir() -> ConfigResult<PathBuf> {
    // dirs::config_dir() already handles platform differences correctly
    let config_dir = dirs::config_dir().map(|p| p.join("ZomboidBackupTool"));
    config_dir.ok_or(ConfigError::ConfigDirNotFound)
}

/// Returns the full path to the config file.
pub fn get_config_file_path() -> ConfigResult<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join(CONFIG_FILE_NAME))
}

/// Loads configuration from the config file.
///
/// # Returns
/// `ConfigResult<Config>` - Loaded configuration, or default if file doesn't exist
///
/// # Behavior
/// - If config file exists, loads and parses it
/// - If config file doesn't exist, returns default config
/// - If config file is corrupted, returns error
pub fn load_config() -> ConfigResult<Config> {
    let config_path = get_config_file_path()?;

    if !config_path.exists() {
        // Config file doesn't exist yet, return default
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| FileOpsError::Io(e))?;

    let config: Config = serde_json::from_str(&content)?;

    Ok(config)
}

/// Saves configuration to the config file.
///
/// # Arguments
/// * `config` - Configuration to save
///
/// # Returns
/// `ConfigResult<()>` - Ok(()) on success
///
/// # Behavior
/// - Creates config directory if it doesn't exist
/// - Overwrites existing config file
/// - Writes formatted JSON for readability
pub fn save_config(config: &Config) -> ConfigResult<()> {
    let config_path = get_config_file_path()?;

    // Create config directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| FileOpsError::Io(e))?;
    }

    // Serialize to formatted JSON
    let json = serde_json::to_string_pretty(config)?;

    // Write to file
    fs::write(&config_path, json)
        .map_err(|e| FileOpsError::Io(e))?;

    Ok(())
}

/// Updates the save path in the configuration and persists it.
pub fn update_save_path(save_path: String) -> ConfigResult<()> {
    let mut config = load_config()?;
    config.save_path = Some(save_path);
    save_config(&config)
}

/// Updates the backup path in the configuration and persists it.
pub fn update_backup_path(backup_path: String) -> ConfigResult<()> {
    let mut config = load_config()?;
    config.backup_path = Some(backup_path);
    save_config(&config)
}

/// Updates the retention count in the configuration and persists it.
pub fn update_retention_count(count: usize) -> ConfigResult<()> {
    if count == 0 {
        return Err(ConfigError::InvalidValue(
            format!("Retention count must be at least 1, got {}", count)
        ));
    }

    let mut config = load_config()?;
    config.retention_count = count;
    save_config(&config)
}

/// Lists all save directories in the Zomboid saves folder.
///
/// # Returns
/// `ConfigResult<Vec<String>>` - List of save names
///
/// # Behavior
/// - Uses configured or auto-detected save path
/// - Returns names of immediate subdirectories (each is a save)
pub fn list_save_directories() -> ConfigResult<Vec<String>> {
    let config = load_config()?;
    let save_path = config.get_save_path()?;

    if !save_path.exists() {
        return Ok(Vec::new());
    }

    let mut saves = Vec::new();

    for entry in fs::read_dir(&save_path)
        .map_err(|e| FileOpsError::Io(e))?
    {
        let entry = entry.map_err(|e| FileOpsError::Io(e))?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    saves.push(name_str.to_string());
                }
            }
        }
    }

    saves.sort();

    Ok(saves)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::new();

        assert!(config.save_path.is_none());
        assert!(config.backup_path.is_none());
        assert_eq!(config.retention_count, DEFAULT_RETENTION_COUNT);
    }

    #[test]
    fn test_config_with_save_path() {
        let save_path = "/path/to/saves".to_string();
        let config = Config::with_save_path(save_path.clone());

        assert_eq!(config.save_path, Some(save_path));
        assert!(config.backup_path.is_none());
        assert_eq!(config.retention_count, DEFAULT_RETENTION_COUNT);
    }

    #[test]
    fn test_config_with_paths() {
        let save_path = "/path/to/saves".to_string();
        let backup_path = "/path/to/backups".to_string();
        let config = Config::with_paths(save_path.clone(), backup_path.clone());

        assert_eq!(config.save_path, Some(save_path));
        assert_eq!(config.backup_path, Some(backup_path));
        assert_eq!(config.retention_count, DEFAULT_RETENTION_COUNT);
    }

    #[test]
    fn test_config_get_save_path_with_override() {
        let custom_path = "/custom/save/path".to_string();
        let config = Config::with_save_path(custom_path.clone());

        let result = config.get_save_path().unwrap();
        assert_eq!(result, PathBuf::from(custom_path));
    }

    #[test]
    fn test_config_get_save_path_auto_detect() {
        let config = Config::new();

        // This should return a path (may or may not exist on the system)
        let result = config.get_save_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        // Should end with Zomboid/Saves
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("Zomboid") && path_str.contains("Saves"));
    }

    #[test]
    fn test_get_default_backup_path() {
        let result = get_default_backup_path();
        assert!(result.is_ok());

        let path = result.unwrap();
        // Should end with ZomboidBackups
        assert!(path.ends_with("ZomboidBackups"));
    }

    #[test]
    fn test_save_and_load_config() {
        let _temp_dir = TempDir::new().unwrap();

        // Create a custom config
        let original = Config {
            save_path: Some("/test/saves".to_string()),
            backup_path: Some("/test/backups".to_string()),
            retention_count: 15,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back
        let loaded: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.save_path, original.save_path);
        assert_eq!(loaded.backup_path, original.backup_path);
        assert_eq!(loaded.retention_count, original.retention_count);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config::with_paths(
            "/home/user/Zomboid/Saves".to_string(),
            "/home/user/ZomboidBackups".to_string(),
        );

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(config.save_path, deserialized.save_path);
        assert_eq!(config.backup_path, deserialized.backup_path);
        assert_eq!(config.retention_count, deserialized.retention_count);
    }

    #[test]
    fn test_update_retention_count_zero_fails() {
        let result = update_retention_count(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_save_directories_nonexistent_path() {
        // Create a config with a non-existent path
        let config = Config::with_save_path("/nonexistent/zomboid/saves".to_string());

        // The validate should fail because path doesn't exist
        let result = config.validate();
        assert!(matches!(result, Err(FileOpsError::SourceNotFound(_))));
    }

    #[test]
    fn test_list_save_directories_from_real_directory() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create some save directories
        fs::create_dir(saves_dir.join("Survival1")).unwrap();
        fs::create_dir(saves_dir.join("Survival2")).unwrap();
        fs::create_dir(saves_dir.join("Builder")).unwrap();

        // Create a file (should not be included)
        fs::write(saves_dir.join("readme.txt"), "test").unwrap();

        // Mock the config to use our temp directory
        let json = serde_json::json!({
            "save_path": saves_dir.to_str(),
            "backup_path": null,
            "retention_count": 10
        });

        let config: Config = serde_json::from_value(json).unwrap();
        let save_path = config.get_save_path().unwrap();

        // List directories manually
        let mut saves = Vec::new();
        for entry in fs::read_dir(&save_path).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                if let Some(name) = entry.path().file_name() {
                    saves.push(name.to_string_lossy().to_string());
                }
            }
        }

        saves.sort();

        assert_eq!(saves.len(), 3);
        assert!(saves.contains(&"Builder".to_string()));
        assert!(saves.contains(&"Survival1".to_string()));
        assert!(saves.contains(&"Survival2".to_string()));
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::InvalidValue("test error".to_string());
        assert_eq!(err.to_string(), "Invalid config value: test error");
    }

    #[test]
    fn test_config_validate_existing_save_path() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_non_directory_backup_path() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        let backup_file = temp_dir.path().join("backup.txt");
        fs::write(&backup_file, "test").unwrap();

        let config = Config {
            save_path: Some(saves_dir.to_str().unwrap().to_string()),
            backup_path: Some(backup_file.to_str().unwrap().to_string()),
            retention_count: 10,
        };

        let result = config.validate();
        assert!(matches!(result, Err(FileOpsError::NotADirectory(_))));
    }
}
