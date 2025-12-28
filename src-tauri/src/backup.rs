//! Core backup logic for Project Zomboid save backup/restore.
//!
//! This module provides:
//! - Backup creation with timestamp generation
//! - Garbage collection for old backups based on retention policy
//! - Backup listing and metadata queries

use crate::file_ops::{copy_dir_recursive, delete_dir_recursive, get_dir_size, FileOpsError, FileOpsResult};
use crate::config::{Config, ConfigError};
use crate::config as config_module;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Backup information returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    /// Name of the backup directory
    pub name: String,
    /// Full path to the backup
    pub path: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Human-readable size string
    pub size_formatted: String,
    /// ISO 8601 timestamp when backup was created
    pub created_at: String,
    /// Name of the save this backup belongs to
    pub save_name: String,
}

/// Result of a backup creation operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupResult {
    /// Path to the created backup
    pub backup_path: String,
    /// Name of the backup directory
    pub backup_name: String,
    /// Number of backups retained after GC
    pub retained_count: usize,
    /// Number of backups deleted by GC
    pub deleted_count: usize,
}

/// Error type for backup operations.
#[derive(Debug)]
pub enum BackupError {
    /// File operation error
    FileOp(FileOpsError),
    /// Config error
    Config(ConfigError),
    /// Save directory not found
    SaveNotFound(String),
    /// Invalid backup name format
    InvalidBackupName(String),
    /// Backup not found
    BackupNotFound(String),
}

impl From<FileOpsError> for BackupError {
    fn from(err: FileOpsError) -> Self {
        BackupError::FileOp(err)
    }
}

impl From<ConfigError> for BackupError {
    fn from(err: ConfigError) -> Self {
        BackupError::Config(err)
    }
}

impl std::fmt::Display for BackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::FileOp(err) => write!(f, "File operation error: {}", err),
            BackupError::Config(err) => write!(f, "Config error: {}", err),
            BackupError::SaveNotFound(name) => write!(f, "Save directory not found: {}", name),
            BackupError::InvalidBackupName(name) => write!(f, "Invalid backup name format: {}", name),
            BackupError::BackupNotFound(name) => write!(f, "Backup not found: {}", name),
        }
    }
}

impl std::error::Error for BackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BackupError::FileOp(err) => Some(err),
            BackupError::Config(err) => Some(err),
            _ => None,
        }
    }
}

impl Serialize for BackupError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Result type for backup operations.
pub type BackupResultT<T> = Result<T, BackupError>;

/// Generates a timestamped backup directory name.
///
/// # Format
/// `{SaveName}_{YYYY-MM-DD}_{HH-mm-ss}`
///
/// # Example
/// ```
/// # use tauri_app_lib::backup::generate_backup_name;
/// let name = generate_backup_name("Survival");
/// // Returns: "Survival_2024-12-28_14-30-45"
/// ```
pub fn generate_backup_name(save_name: &str) -> String {
    let now = Utc::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S");
    format!("{}_{}", save_name, timestamp)
}

/// Gets the backup directory for a specific save.
///
/// # Arguments
/// * `backup_base_path` - Base backup directory
/// * `save_name` - Name of the save
///
/// # Returns
/// Path to the save's backup subdirectory
pub fn get_save_backup_dir(backup_base_path: &Path, save_name: &str) -> PathBuf {
    backup_base_path.join(save_name)
}

/// Creates a backup of the specified save directory.
///
/// # Arguments
/// * `save_name` - Name of the save to backup (must exist in save path)
///
/// # Returns
/// `BackupResultT<BackupResult>` - Information about the created backup
///
/// # Behavior
/// 1. Validates the save directory exists
/// 2. Generates timestamped backup name
/// 3. Copies the entire save directory to backup location
/// 4. Runs garbage collection to remove old backups exceeding retention limit
pub fn create_backup(save_name: &str) -> BackupResultT<BackupResult> {
    let config = config_module::load_config()?;
    let save_path = config.get_save_path()?;
    let backup_base_path = config.get_backup_path()?;

    // Validate save directory exists
    let save_dir = save_path.join(save_name);
    if !save_dir.exists() {
        return Err(BackupError::SaveNotFound(save_name.to_string()));
    }
    if !save_dir.is_dir() {
        return Err(BackupError::SaveNotFound(format!("{} is not a directory", save_name)));
    }

    // Create backup base directory if it doesn't exist
    let save_backup_dir = get_save_backup_dir(&backup_base_path, save_name);
    if !save_backup_dir.exists() {
        fs::create_dir_all(&save_backup_dir)
            .map_err(|e| FileOpsError::Io(e))?;
    }

    // Generate backup name and path
    let backup_name = generate_backup_name(save_name);
    let backup_path = save_backup_dir.join(&backup_name);

    // Perform the backup copy
    copy_dir_recursive(&save_dir, &backup_path)?;

    // Run garbage collection
    let retention_count = config.retention_count;
    let (retained, deleted) = garbage_collection(&save_backup_dir, retention_count)?;

    Ok(BackupResult {
        backup_path: backup_path.to_string_lossy().to_string(),
        backup_name,
        retained_count: retained,
        deleted_count: deleted,
    })
}

/// Performs garbage collection on old backups.
///
/// # Arguments
/// * `save_backup_dir` - Directory containing backups for a specific save
/// * `retention_count` - Maximum number of backups to retain
///
/// # Returns
/// `FileOpsResult<(usize, usize)>` - (retained_count, deleted_count)
///
/// # Behavior
/// - Lists all backup directories sorted by creation time (newest first)
/// - Keeps the newest `retention_count` backups
/// - Deletes older backups
fn garbage_collection(save_backup_dir: &Path, retention_count: usize) -> FileOpsResult<(usize, usize)> {
    let mut backups = list_backup_dirs(save_backup_dir)?;

    // Sort by creation time (newest first)
    backups.sort_by(|a, b| b.created.cmp(&a.created));

    let total_backups = backups.len();
    let to_delete = if total_backups > retention_count {
        backups.split_off(retention_count)
    } else {
        Vec::new()
    };

    // Delete old backups
    for backup in &to_delete {
        let backup_path = save_backup_dir.join(&backup.name);
        // Silently ignore errors during GC - a failed deletion is not critical
        let _ = delete_dir_recursive(&backup_path);
    }

    let retained = total_backups.saturating_sub(to_delete.len());
    let deleted = to_delete.len();

    Ok((retained, deleted))
}

/// Internal struct for tracking backup directories during GC.
#[derive(Debug)]
struct BackupDir {
    name: String,
    created: SystemTime,
}

/// Lists all backup directories in a save's backup folder.
///
/// # Arguments
/// * `save_backup_dir` - Directory containing backups for a specific save
///
/// # Returns
/// `FileOpsResult<Vec<BackupDir>>` - List of backup directories with metadata
fn list_backup_dirs(save_backup_dir: &Path) -> FileOpsResult<Vec<BackupDir>> {
    if !save_backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(save_backup_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    let metadata = entry.metadata()?;
                    let created = metadata.created()
                        .or_else(|_| metadata.modified())
                        .unwrap_or_else(|_| SystemTime::now());

                    backups.push(BackupDir {
                        name: name_str.to_string(),
                        created,
                    });
                }
            }
        }
    }

    Ok(backups)
}

/// Lists all backups for a specific save.
///
/// # Arguments
/// * `save_name` - Name of the save
///
/// # Returns
/// `BackupResultT<Vec<BackupInfo>>` - List of backups sorted by creation time (newest first)
pub fn list_backups(save_name: &str) -> BackupResultT<Vec<BackupInfo>> {
    let config = config_module::load_config()?;
    let backup_base_path = config.get_backup_path()?;
    let save_backup_dir = get_save_backup_dir(&backup_base_path, save_name);

    if !save_backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&save_backup_dir)
        .map_err(|e| FileOpsError::Io(e))?
    {
        let entry = entry.map_err(|e| FileOpsError::Io(e))?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    let size_bytes = get_dir_size(&path)?;
                    let size_formatted = crate::file_ops::format_size(size_bytes);

                    // Get creation time
                    let metadata = entry.metadata().map_err(|e| FileOpsError::Io(e))?;
                    let created = metadata.created()
                        .or_else(|_| metadata.modified())
                        .unwrap_or_else(|_| SystemTime::now());
                    let created_dt: DateTime<Utc> = created.into();
                    let created_at = created_dt.to_rfc3339();

                    backups.push(BackupInfo {
                        name: name_str.to_string(),
                        path: path.to_string_lossy().to_string(),
                        size_bytes,
                        size_formatted,
                        created_at,
                        save_name: save_name.to_string(),
                    });
                }
            }
        }
    }

    // Sort by creation time (newest first)
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(backups)
}

/// Gets detailed information about a specific backup.
///
/// # Arguments
/// * `save_name` - Name of the save
/// * `backup_name` - Name of the backup directory
///
/// # Returns
/// `BackupResultT<BackupInfo>` - Detailed backup information
pub fn get_backup_info(save_name: &str, backup_name: &str) -> BackupResultT<BackupInfo> {
    let config = config_module::load_config()?;
    let backup_base_path = config.get_backup_path()?;
    let save_backup_dir = get_save_backup_dir(&backup_base_path, save_name);
    let backup_path = save_backup_dir.join(backup_name);

    if !backup_path.exists() {
        return Err(BackupError::BackupNotFound(
            format!("{}/{}", save_name, backup_name)
        ));
    }

    let size_bytes = get_dir_size(&backup_path)?;
    let size_formatted = crate::file_ops::format_size(size_bytes);

    let metadata = fs::metadata(&backup_path)
        .map_err(|e| FileOpsError::Io(e))?;
    let created = metadata.created()
        .or_else(|_| metadata.modified())
        .unwrap_or_else(|_| SystemTime::now());
    let created_dt: DateTime<Utc> = created.into();
    let created_at = created_dt.to_rfc3339();

    Ok(BackupInfo {
        name: backup_name.to_string(),
        path: backup_path.to_string_lossy().to_string(),
        size_bytes,
        size_formatted,
        created_at,
        save_name: save_name.to_string(),
    })
}

/// Lists all saves that have at least one backup.
///
/// # Returns
/// `BackupResultT<Vec<String>>` - List of save names with backups
pub fn list_saves_with_backups() -> BackupResultT<Vec<String>> {
    let config = config_module::load_config()?;
    let backup_base_path = config.get_backup_path()?;

    if !backup_base_path.exists() {
        return Ok(Vec::new());
    }

    let mut saves = Vec::new();

    for entry in fs::read_dir(&backup_base_path)
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

/// Counts the number of backups for a specific save.
///
/// # Arguments
/// * `save_name` - Name of the save
///
/// # Returns
/// `BackupResultT<usize>` - Number of backups
pub fn count_backups(save_name: &str) -> BackupResultT<usize> {
    let backups = list_backups(save_name)?;
    Ok(backups.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config as config_module;
    use serial_test::serial;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to create a test save directory with files
    fn create_test_save(save_dir: &Path) {
        fs::create_dir_all(save_dir.join("map")).unwrap();
        File::create(save_dir.join("save.bin")).unwrap().write_all(b"game state").unwrap();
        File::create(save_dir.join("map/pchunk_0_0.dat")).unwrap().write_all(b"map data").unwrap();
        File::create(save_dir.join("map/pchunk_0_1.dat")).unwrap().write_all(b"more map").unwrap();
    }

    /// Helper to setup test config
    fn setup_test_config(save_dir: &Path, backup_dir: &Path) {
        let config = Config::with_paths(
            save_dir.to_str().unwrap().to_string(),
            backup_dir.to_str().unwrap().to_string(),
        );
        config_module::save_config(&config).unwrap();
    }

    #[test]
    fn test_generate_backup_name_format() {
        let name = generate_backup_name("Survival");
        // Format: {SaveName}_{YYYY-MM-DD}_{HH-mm-ss}
        assert!(name.starts_with("Survival_"));
        let parts: Vec<&str> = name.split('_').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[1].chars().filter(|&c| c == '-').count() == 2); // Date has 2 dashes
        assert!(parts[2].chars().filter(|&c| c == '-').count() == 2); // Time has 2 dashes
    }

    #[test]
    fn test_get_save_backup_dir() {
        let base = Path::new("/backups");
        let save_dir = get_save_backup_dir(base, "Survival");
        assert_eq!(save_dir, Path::new("/backups/Survival"));
    }

    #[test]
    fn test_list_backup_dirs_empty() {
        let temp_dir = TempDir::new().unwrap();
        let backups = list_backup_dirs(temp_dir.path()).unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[test]
    fn test_list_backup_dirs_with_backups() {
        let temp_dir = TempDir::new().unwrap();
        let backup1 = temp_dir.path().join("Survival_2024-12-28_10-00-00");
        let backup2 = temp_dir.path().join("Survival_2024-12-28_11-00-00");

        fs::create_dir(&backup1).unwrap();
        fs::create_dir(&backup2).unwrap();

        // Create files to ensure directories have content
        File::create(backup1.join("save.bin")).unwrap().write_all(b"data").unwrap();
        File::create(backup2.join("save.bin")).unwrap().write_all(b"data").unwrap();

        let backups = list_backup_dirs(temp_dir.path()).unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[test]
    #[serial]
    fn test_create_backup_success() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        let result = create_backup("Survival").unwrap();
        assert!(result.backup_path.contains("Survival_"));
        assert!(result.backup_name.starts_with("Survival_"));
        assert_eq!(result.retained_count, 1);
        assert_eq!(result.deleted_count, 0);
    }

    #[test]
    #[serial]
    fn test_create_backup_save_not_found() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let result = create_backup("NonExistent");
        assert!(matches!(result, Err(BackupError::SaveNotFound(_))));
    }

    #[test]
    fn test_garbage_collection_with_retention_limit() {
        let temp_dir = TempDir::new().unwrap();

        // Create 5 backup directories
        for i in 0..5 {
            let backup_path = temp_dir.path().join(format!("Survival_2024-12-28_{:02}-00-00", i));
            fs::create_dir(&backup_path).unwrap();
            File::create(backup_path.join("save.bin")).unwrap().write_all(b"data").unwrap();
        }

        // Set retention to 3
        let (retained, deleted) = garbage_collection(temp_dir.path(), 3).unwrap();

        assert_eq!(retained, 3);
        assert_eq!(deleted, 2);

        // Verify only 3 backups remain
        let remaining = list_backup_dirs(temp_dir.path()).unwrap();
        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn test_garbage_collection_no_deletion_needed() {
        let temp_dir = TempDir::new().unwrap();

        // Create 2 backup directories
        for i in 0..2 {
            let backup_path = temp_dir.path().join(format!("Survival_2024-12-28_{:02}-00-00", i));
            fs::create_dir(&backup_path).unwrap();
            File::create(backup_path.join("save.bin")).unwrap().write_all(b"data").unwrap();
        }

        // Set retention to 5 (more than existing)
        let (retained, deleted) = garbage_collection(temp_dir.path(), 5).unwrap();

        assert_eq!(retained, 2);
        assert_eq!(deleted, 0);

        // Verify all backups remain
        let remaining = list_backup_dirs(temp_dir.path()).unwrap();
        assert_eq!(remaining.len(), 2);
    }

    #[test]
    #[serial]
    fn test_list_backups_empty() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let backups = list_backups("Survival").unwrap();
        assert_eq!(backups.len(), 0);
    }

    #[test]
    #[serial]
    fn test_list_backups_with_data() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        // Create a backup
        create_backup("Survival").unwrap();

        let backups = list_backups("Survival").unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].save_name, "Survival");
        assert!(backups[0].name.starts_with("Survival_"));
        assert!(backups[0].size_bytes > 0);
        assert!(!backups[0].size_formatted.is_empty());
    }

    #[test]
    #[serial]
    fn test_get_backup_info_success() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        let backup_result = create_backup("Survival").unwrap();
        let backup_name = backup_result.backup_name;

        // Verify the backup was actually created with files
        let backup_path = backup_base.path().join("Survival").join(&backup_name);
        assert!(backup_path.exists());
        assert!(backup_path.join("save.bin").exists());

        let info = get_backup_info("Survival", &backup_name).unwrap();
        assert_eq!(info.name, backup_name);
        assert_eq!(info.save_name, "Survival");
        assert!(info.size_bytes > 0);
    }

    #[test]
    #[serial]
    fn test_get_backup_info_not_found() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let result = get_backup_info("Survival", "NonExistent");
        assert!(matches!(result, Err(BackupError::BackupNotFound(_))));
    }

    #[test]
    #[serial]
    fn test_count_backups() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        assert_eq!(count_backups("Survival").unwrap(), 0);

        create_backup("Survival").unwrap();
        assert_eq!(count_backups("Survival").unwrap(), 1);

        // Add delay to ensure different timestamps (backup names have second precision)
        std::thread::sleep(std::time::Duration::from_secs(2));
        create_backup("Survival").unwrap();
        assert_eq!(count_backups("Survival").unwrap(), 2);
    }

    #[test]
    #[serial]
    fn test_list_saves_with_backups() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        // Create saves for two different games
        let survival_dir = save_base.path().join("Survival");
        let builder_dir = save_base.path().join("Builder");

        create_test_save(&survival_dir);
        create_test_save(&builder_dir);

        create_backup("Survival").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        create_backup("Builder").unwrap();

        let saves = list_saves_with_backups().unwrap();
        assert_eq!(saves.len(), 2);
        assert!(saves.contains(&"Builder".to_string()));
        assert!(saves.contains(&"Survival".to_string()));
    }

    #[test]
    fn test_backup_result_serialization() {
        let result = BackupResult {
            backup_path: "/backups/Survival_2024-12-28_10-00-00".to_string(),
            backup_name: "Survival_2024-12-28_10-00-00".to_string(),
            retained_count: 5,
            deleted_count: 2,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: BackupResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.backup_path, result.backup_path);
        assert_eq!(parsed.backup_name, result.backup_name);
        assert_eq!(parsed.retained_count, 5);
        assert_eq!(parsed.deleted_count, 2);
    }

    #[test]
    fn test_backup_info_serialization() {
        let info = BackupInfo {
            name: "Survival_2024-12-28_10-00-00".to_string(),
            path: "/backups/Survival/Survival_2024-12-28_10-00-00".to_string(),
            size_bytes: 1024,
            size_formatted: "1.00 KB".to_string(),
            created_at: "2024-12-28T10:00:00Z".to_string(),
            save_name: "Survival".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: BackupInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, info.name);
        assert_eq!(parsed.size_bytes, 1024);
        assert_eq!(parsed.save_name, "Survival");
    }

    #[test]
    fn test_backup_error_display() {
        let err = BackupError::SaveNotFound("TestSave".to_string());
        assert_eq!(err.to_string(), "Save directory not found: TestSave");

        let err2 = BackupError::InvalidBackupName("bad_name".to_string());
        assert_eq!(err2.to_string(), "Invalid backup name format: bad_name");
    }

    #[test]
    #[serial]
    fn test_multiple_backups_with_gc() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        // Set retention to 3
        let config = Config::with_paths(
            save_base.path().to_str().unwrap().to_string(),
            backup_base.path().to_str().unwrap().to_string(),
        );
        let config_with_retention = Config {
            retention_count: 3,
            ..config
        };
        config_module::save_config(&config_with_retention).unwrap();

        // Create 5 backups
        for _ in 0..5 {
            create_backup("Survival").unwrap();
            // Delay to ensure different timestamps (backup names have second precision)
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        // Should only have 3 backups due to GC
        let count = count_backups("Survival").unwrap();
        assert_eq!(count, 3);

        let backups = list_backups("Survival").unwrap();
        assert_eq!(backups.len(), 3);
    }
}
