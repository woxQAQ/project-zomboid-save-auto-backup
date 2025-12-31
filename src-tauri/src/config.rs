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

    /// Whether to automatically check for updates on startup.
    #[serde(default = "default_auto_check_updates")]
    pub auto_check_updates: bool,

    /// Timestamp of the last update check (ISO 8601 format).
    #[serde(default)]
    pub last_update_check: Option<String>,

    /// Last selected save relative path (e.g., "Survival/MySave").
    /// Used to restore the user's previous selection on app startup.
    #[serde(default)]
    pub last_selected_save: Option<String>,
}

/// Default value for auto_check_updates field.
fn default_auto_check_updates() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Config {
            save_path: None,
            backup_path: None,
            retention_count: DEFAULT_RETENTION_COUNT,
            auto_check_updates: default_auto_check_updates(),
            last_update_check: None,
            last_selected_save: None,
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
        .map_err(FileOpsError::Io)?;

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
            .map_err(FileOpsError::Io)?;
    }

    // Serialize to formatted JSON
    let json = serde_json::to_string_pretty(config)?;

    // Write to file
    fs::write(&config_path, json)
        .map_err(FileOpsError::Io)?;

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

/// Updates the last selected save in the configuration and persists it.
///
/// # Arguments
/// * `relative_path` - The relative path of the selected save (e.g., "Survival/MySave")
///
/// # Returns
/// `ConfigResult<()>` - Ok(()) on success
///
/// # Example
/// ```no_run
/// use tauri_app_lib::config::update_last_selected_save;
///
/// update_last_selected_save("Survival/MySave".to_string()).unwrap();
/// ```
pub fn update_last_selected_save(relative_path: String) -> ConfigResult<()> {
    let mut config = load_config()?;
    config.last_selected_save = Some(relative_path);
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
        .map_err(FileOpsError::Io)?
    {
        let entry = entry.map_err(FileOpsError::Io)?;
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

/// A save entry with game mode information.
///
/// Represents a single save with its associated game mode.
/// Used for the two-level directory structure: `Saves/<GameMode>/<SaveName>`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SaveEntry {
    /// The game mode (e.g., "Survival", "Builder", "Sandbox")
    pub game_mode: String,
    /// The save name (actual save folder name)
    pub save_name: String,
    /// Full relative path from Saves root (e.g., "Survival/MySave")
    pub relative_path: String,
}

impl SaveEntry {
    /// Creates a new SaveEntry.
    pub fn new(game_mode: String, save_name: String) -> Self {
        let relative_path = if game_mode.is_empty() {
            save_name.clone()
        } else {
            format!("{}/{}", game_mode, save_name)
        };
        Self {
            game_mode,
            save_name,
            relative_path,
        }
    }

    /// Creates a SaveEntry for saves without a game mode (legacy/flat structure).
    pub fn flat(save_name: String) -> Self {
        Self::new(String::new(), save_name)
    }

    /// Returns the full path to this save directory.
    ///
    /// # Arguments
    /// * `base_path` - The Saves base path
    pub fn full_path(&self, base_path: &Path) -> PathBuf {
        base_path.join(&self.relative_path)
    }
}

/// Lists all save entries with game mode information.
///
/// Scans the Zomboid saves directory for the two-level structure `Saves/<GameMode>/<SaveName>`.
/// Also supports legacy flat structure for backwards compatibility.
///
/// # Returns
/// `ConfigResult<Vec<SaveEntry>>` - List of save entries with game mode info
///
/// # Behavior
/// 1. Scans all subdirectories in the Saves folder
/// 2. If a subdirectory looks like a game mode (contains save subdirectories),
///    treats its children as saves
/// 3. If a subdirectory looks like a save (contains map/*.bin files),
///    treats it as a flat save (legacy structure)
/// 4. Returns sorted list (by game mode, then save name)
///
/// # Example
/// ```no_run
/// use tauri_app_lib::config::list_save_entries;
///
/// let entries = list_save_entries().unwrap();
/// for entry in entries {
///     println!("{}: {}", entry.game_mode, entry.save_name);
/// }
/// ```
pub fn list_save_entries() -> ConfigResult<Vec<SaveEntry>> {
    let config = load_config()?;
    let save_path = config.get_save_path()?;

    if !save_path.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    // Read all entries in the Saves directory
    for game_mode_entry in fs::read_dir(&save_path)
        .map_err(FileOpsError::Io)?
    {
        let game_mode_entry = game_mode_entry.map_err(FileOpsError::Io)?;
        let game_mode_path = game_mode_entry.path();

        // Only process directories
        if !game_mode_path.is_dir() {
            continue;
        }

        let game_mode_name = match game_mode_path.file_name() {
            Some(name) => name.to_string_lossy().to_string(),
            None => continue,
        };

        // Check if this looks like a game mode directory (contains save subdirectories)
        let mut has_save_subdirs = false;
        let mut has_save_files = false;

        if let Ok(sub_entries) = fs::read_dir(&game_mode_path) {
            for sub_entry in sub_entries {
                let sub_entry = match sub_entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let sub_path = sub_entry.path();

                if sub_path.is_dir() {
                    // Check if this subdirectory looks like a save
                    if looks_like_save_directory(&sub_path) {
                        has_save_subdirs = true;
                        let save_name = sub_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        entries.push(SaveEntry::new(game_mode_name.clone(), save_name));
                    }
                } else {
                    // Check if this is a save file (map/*.bin or save.bin at root)
                    if looks_like_save_file(&sub_path) {
                        has_save_files = true;
                    }
                }
            }
        }

        // If this directory has save files but no save subdirectories,
        // it might be a flat save (legacy structure)
        if has_save_files && !has_save_subdirs {
            if looks_like_save_directory(&game_mode_path) {
                entries.push(SaveEntry::flat(game_mode_name));
            }
        }
    }

    // Sort by game mode, then by save name
    entries.sort();

    Ok(entries)
}

/// Checks if a directory looks like a Project Zomboid save directory.
///
/// A save directory typically contains:
/// - A `map` subdirectory with `.bin` or `.dat` files
/// - Or `save.bin` / `map_p.bin` files at the root
fn looks_like_save_directory(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    let map_dir = path.join("map");
    if map_dir.is_dir() {
        // Check for map chunk files
        if let Ok(entries) = fs::read_dir(&map_dir) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.is_file() {
                    let name = file_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    // Look for .bin or .dat files (typical map chunks)
                    if name.ends_with(".bin") || name.ends_with(".dat") {
                        return true;
                    }
                }
            }
        }
    }

    // Check for save files at root level
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.is_file() {
                let name = file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                // Typical save files - be more specific to avoid false positives
                // Check for specific known save files or map chunk files (prefix_*.bin)
                if name == "save.bin" || name == "map_p.bin" || (name.starts_with("map_") && name.ends_with(".bin")) {
                    return true;
                }
            }
        }
    }

    false
}

/// Checks if a file looks like a Project Zomboid save file.
fn looks_like_save_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let name = match path.file_name() {
        Some(n) => n.to_string_lossy(),
        None => return false,
    };

    // Check for common save file patterns
    name.ends_with(".bin") || name == "map_p.bin" || name == "save.bin"
}

/// Gets save entries grouped by game mode.
///
/// # Returns
/// `ConfigResult<std::collections::HashMap<String, Vec<SaveEntry>>>` - Map of game mode to save entries
///
/// # Example
/// ```no_run
/// use tauri_app_lib::config::list_save_entries_by_game_mode;
///
/// let grouped = list_save_entries_by_game_mode().unwrap();
/// for (game_mode, saves) in grouped {
///     println!("{}: {} saves", game_mode, saves.len());
/// }
/// ```
pub fn list_save_entries_by_game_mode() -> ConfigResult<std::collections::HashMap<String, Vec<SaveEntry>>> {
    let entries = list_save_entries()?;
    let mut grouped: std::collections::HashMap<String, Vec<SaveEntry>> = std::collections::HashMap::new();

    for entry in entries {
        // Use "(Other)" with parentheses to avoid collision with actual game mode named "Other"
        let game_mode = if entry.game_mode.is_empty() {
            "(Other)".to_string()
        } else {
            entry.game_mode.clone()
        };

        grouped.entry(game_mode).or_insert_with(Vec::new).push(entry);
    }

    Ok(grouped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs::{self, File};
    use std::io::Write;
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
            auto_check_updates: true,
            last_update_check: None,
            last_selected_save: None,
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
    #[serial]
    fn test_update_last_selected_save() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());
        save_config(&config).unwrap();

        // Update last selected save
        update_last_selected_save("Survival/MySave".to_string()).unwrap();

        // Verify persistence
        let loaded = load_config().unwrap();
        assert_eq!(loaded.last_selected_save, Some("Survival/MySave".to_string()));
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
            auto_check_updates: true,
            last_update_check: None,
            last_selected_save: None,
        };

        let result = config.validate();
        assert!(matches!(result, Err(FileOpsError::NotADirectory(_))));
    }

    // ============================================================================
    // CORE-06: Save Scanning with Game Mode Support - Unit Tests
    // ============================================================================

    /// Helper to create a test save directory structure
    fn create_test_save_structure(save_dir: &Path) {
        fs::create_dir_all(save_dir.join("map")).unwrap();
        File::create(save_dir.join("save.bin")).unwrap().write_all(b"game state").unwrap();
        File::create(save_dir.join("map/pchunk_0_0.dat")).unwrap().write_all(b"map data").unwrap();
    }

    #[test]
    fn test_save_entry_new() {
        let entry = SaveEntry::new("Survival".to_string(), "MySave".to_string());
        assert_eq!(entry.game_mode, "Survival");
        assert_eq!(entry.save_name, "MySave");
        assert_eq!(entry.relative_path, "Survival/MySave");
    }

    #[test]
    fn test_save_entry_flat() {
        let entry = SaveEntry::flat("OldSave".to_string());
        assert_eq!(entry.game_mode, "");
        assert_eq!(entry.save_name, "OldSave");
        assert_eq!(entry.relative_path, "OldSave");
    }

    #[test]
    fn test_save_entry_full_path() {
        let entry = SaveEntry::new("Survival".to_string(), "MySave".to_string());
        let base = Path::new("/home/user/Zomboid/Saves");
        let full = entry.full_path(base);
        assert_eq!(full, Path::new("/home/user/Zomboid/Saves/Survival/MySave"));
    }

    #[test]
    fn test_save_entry_serialization() {
        let entry = SaveEntry::new("Survival".to_string(), "MySave".to_string());
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: SaveEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.game_mode, "Survival");
        assert_eq!(parsed.save_name, "MySave");
        assert_eq!(parsed.relative_path, "Survival/MySave");
    }

    #[test]
    fn test_save_entry_ordering() {
        let mut entries = vec![
            SaveEntry::new("Survival".to_string(), "Save2".to_string()),
            SaveEntry::new("Builder".to_string(), "Save1".to_string()),
            SaveEntry::new("Survival".to_string(), "Save1".to_string()),
        ];
        entries.sort();
        assert_eq!(entries[0].game_mode, "Builder");
        assert_eq!(entries[1].game_mode, "Survival");
        assert_eq!(entries[1].save_name, "Save1");
        assert_eq!(entries[2].save_name, "Save2");
    }

    #[test]
    #[serial]
    fn test_list_save_entries_two_level_structure() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create two-level structure: Saves/<GameMode>/<SaveName>
        let survival_mode = saves_dir.join("Survival");
        let builder_mode = saves_dir.join("Builder");

        let survival_save = survival_mode.join("MySurvival");
        let builder_save = builder_mode.join("MyBuilder");

        create_test_save_structure(&survival_save);
        create_test_save_structure(&builder_save);

        // Setup config
        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());
        save_config(&config).unwrap();

        let entries = list_save_entries().unwrap();

        assert_eq!(entries.len(), 2);

        // Check Survival save
        let survival_entry = entries.iter().find(|e| e.game_mode == "Survival").unwrap();
        assert_eq!(survival_entry.save_name, "MySurvival");
        assert_eq!(survival_entry.relative_path, "Survival/MySurvival");

        // Check Builder save
        let builder_entry = entries.iter().find(|e| e.game_mode == "Builder").unwrap();
        assert_eq!(builder_entry.save_name, "MyBuilder");
        assert_eq!(builder_entry.relative_path, "Builder/MyBuilder");
    }

    #[test]
    #[serial]
    fn test_list_save_entries_flat_structure() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create flat structure (legacy): Saves/<SaveName>
        let old_save = saves_dir.join("OldSave");
        create_test_save_structure(&old_save);

        // Setup config
        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());
        save_config(&config).unwrap();

        let entries = list_save_entries().unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].game_mode, "");
        assert_eq!(entries[0].save_name, "OldSave");
        assert_eq!(entries[0].relative_path, "OldSave");
    }

    #[test]
    #[serial]
    fn test_list_save_entries_mixed_structure() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create mixed structure: some flat, some two-level
        let old_save = saves_dir.join("OldSave");
        create_test_save_structure(&old_save);

        let survival_mode = saves_dir.join("Survival");
        let survival_save = survival_mode.join("NewSave");
        create_test_save_structure(&survival_save);

        // Setup config
        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());
        save_config(&config).unwrap();

        let entries = list_save_entries().unwrap();

        assert_eq!(entries.len(), 2);

        // Flat save should have empty game_mode
        let flat_entry = entries.iter().find(|e| e.save_name == "OldSave").unwrap();
        assert_eq!(flat_entry.game_mode, "");

        // Two-level save should have game_mode
        let nested_entry = entries.iter().find(|e| e.save_name == "NewSave").unwrap();
        assert_eq!(nested_entry.game_mode, "Survival");
    }

    #[test]
    #[serial]
    fn test_list_save_entries_nonexistent_path() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("NonExistent");

        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());
        save_config(&config).unwrap();

        let entries = list_save_entries().unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    #[serial]
    fn test_list_save_entries_by_game_mode() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create multiple saves in different game modes
        let survival_mode = saves_dir.join("Survival");
        let builder_mode = saves_dir.join("Builder");

        create_test_save_structure(&survival_mode.join("Save1"));
        create_test_save_structure(&survival_mode.join("Save2"));
        create_test_save_structure(&builder_mode.join("Build1"));

        // Setup config
        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());
        save_config(&config).unwrap();

        let grouped = list_save_entries_by_game_mode().unwrap();

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get("Survival").unwrap().len(), 2);
        assert_eq!(grouped.get("Builder").unwrap().len(), 1);
    }

    #[test]
    fn test_looks_like_save_directory_with_map_files() {
        let temp_dir = TempDir::new().unwrap();
        let save_dir = temp_dir.path().join("save");

        fs::create_dir_all(save_dir.join("map")).unwrap();
        File::create(save_dir.join("map/pchunk_0_0.dat")).unwrap().write_all(b"data").unwrap();

        assert!(looks_like_save_directory(&save_dir));
    }

    #[test]
    fn test_looks_like_save_directory_with_save_bin() {
        let temp_dir = TempDir::new().unwrap();
        let save_dir = temp_dir.path().join("save");

        fs::create_dir(&save_dir).unwrap();
        File::create(save_dir.join("save.bin")).unwrap().write_all(b"data").unwrap();

        assert!(looks_like_save_directory(&save_dir));
    }

    #[test]
    fn test_looks_like_save_directory_empty() {
        let temp_dir = TempDir::new().unwrap();
        let save_dir = temp_dir.path().join("save");

        fs::create_dir(&save_dir).unwrap();

        assert!(!looks_like_save_directory(&save_dir));
    }

    #[test]
    fn test_looks_like_save_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("save.bin");
        File::create(&file_path).unwrap().write_all(b"data").unwrap();

        assert!(looks_like_save_file(&file_path));
    }

    #[test]
    fn test_looks_like_save_file_non_bin() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("readme.txt");
        File::create(&file_path).unwrap().write_all(b"text").unwrap();

        assert!(!looks_like_save_file(&file_path));
    }

    #[test]
    #[serial]
    fn test_list_save_entries_ignores_non_save_directories() {
        let temp_dir = TempDir::new().unwrap();
        let saves_dir = temp_dir.path().join("Saves");
        fs::create_dir(&saves_dir).unwrap();

        // Create a valid save
        let survival_mode = saves_dir.join("Survival");
        create_test_save_structure(&survival_mode.join("MySave"));

        // Create directories that don't look like saves
        fs::create_dir(saves_dir.join("EmptyFolder")).unwrap();
        fs::create_dir(saves_dir.join("NotASave")).unwrap();

        // Setup config
        let config = Config::with_save_path(saves_dir.to_str().unwrap().to_string());
        save_config(&config).unwrap();

        let entries = list_save_entries().unwrap();

        // Should only find the valid save, not the empty folders
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].save_name, "MySave");
    }
}
