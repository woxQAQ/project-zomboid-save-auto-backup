//! Safe restore logic for Project Zomboid save backup/restore.
//!
//! This module provides:
//! - Safe restore with undo snapshot creation
//! - Pre-restore backup of current save state
//! - Atomic restore operations with rollback capability

use crate::backup::{get_save_backup_dir, BackupError, BackupResultT};
use crate::config as config_module;
use crate::config::ConfigError;
use crate::file_ops::{copy_dir_recursive, delete_dir_recursive, FileOpsError, FileOpsResult};
use serde::{Deserialize, Serialize, Serializer};
use std::fs;
use std::path::{Path, PathBuf};

/// Result of a restore operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreResult {
    /// Path to the save that was restored
    pub save_path: String,
    /// Name of the save that was restored
    pub save_name: String,
    /// Path to the backup that was restored
    pub backup_path: String,
    /// Name of the backup that was restored
    pub backup_name: String,
    /// Path to the undo snapshot (if created)
    pub undo_snapshot_path: Option<String>,
    /// Whether an undo snapshot was created
    pub has_undo_snapshot: bool,
}

/// Information about an undo snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoSnapshotInfo {
    /// Name of the undo snapshot directory
    pub name: String,
    /// Full path to the undo snapshot
    pub path: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Human-readable size string
    pub size_formatted: String,
    /// ISO 8601 timestamp when snapshot was created
    pub created_at: String,
    /// Name of the save this snapshot belongs to
    pub save_name: String,
}

/// Error type for restore operations.
#[derive(Debug)]
pub enum RestoreError {
    /// File operation error
    FileOp(FileOpsError),
    /// Backup error
    Backup(BackupError),
    /// Config error
    Config(ConfigError),
    /// Save directory not found
    SaveNotFound(String),
    /// Backup not found
    BackupNotFound(String),
    /// Current save not found (nothing to snapshot before restore)
    CurrentSaveNotFound(String),
    /// Undo snapshot directory creation failed
    UndoSnapshotFailed(String),
}

impl From<FileOpsError> for RestoreError {
    fn from(err: FileOpsError) -> Self {
        RestoreError::FileOp(err)
    }
}

impl From<BackupError> for RestoreError {
    fn from(err: BackupError) -> Self {
        RestoreError::Backup(err)
    }
}

impl From<ConfigError> for RestoreError {
    fn from(err: ConfigError) -> Self {
        RestoreError::Config(err)
    }
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreError::FileOp(err) => write!(f, "File operation error: {}", err),
            RestoreError::Backup(err) => write!(f, "Backup error: {}", err),
            RestoreError::Config(err) => write!(f, "Config error: {}", err),
            RestoreError::SaveNotFound(name) => write!(f, "Save directory not found: {}", name),
            RestoreError::BackupNotFound(name) => write!(f, "Backup not found: {}", name),
            RestoreError::CurrentSaveNotFound(name) => {
                write!(f, "Current save not found (nothing to snapshot): {}", name)
            }
            RestoreError::UndoSnapshotFailed(msg) => {
                write!(f, "Failed to create undo snapshot: {}", msg)
            }
        }
    }
}

impl std::error::Error for RestoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RestoreError::FileOp(err) => Some(err),
            RestoreError::Backup(err) => Some(err),
            RestoreError::Config(err) => Some(err),
            _ => None,
        }
    }
}

impl Serialize for RestoreError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Result type for restore operations.
pub type RestoreResultT<T> = Result<T, RestoreError>;

/// Gets the undo snapshot directory for a specific save.
///
/// # Arguments
/// * `backup_base_path` - Base backup directory
/// * `save_name` - Name of the save
///
/// # Returns
/// Path to the save's undo snapshot subdirectory
pub fn get_undo_snapshot_dir(backup_base_path: &Path, save_name: &str) -> PathBuf {
    backup_base_path.join(format!("{}_undo", save_name))
}

/// Generates a timestamped undo snapshot name.
///
/// # Format
/// `undo_{YYYY-MM-DD}_{HH-mm-ss}`
///
/// # Example
/// ```no_run
/// let name = generate_undo_snapshot_name();
/// // Returns: "undo_2024-12-28_14-30-45"
/// ```
pub fn generate_undo_snapshot_name() -> String {
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%d_%H-%M-%S");
    format!("undo_{}", timestamp)
}

/// Creates an undo snapshot of the current save state.
///
/// # Arguments
/// * `save_path` - Path to the current save directory
/// * `undo_snapshot_dir` - Directory to store undo snapshots
///
/// # Returns
/// `RestoreResultT<UndoSnapshotInfo>` - Information about the created snapshot
///
/// # Behavior
/// - Creates a timestamped snapshot of the current save
/// - Returns Ok(None) if save doesn't exist (nothing to snapshot)
/// - If snapshot with same name exists, deletes it first before creating new one
fn create_undo_snapshot(
    save_path: &Path,
    undo_snapshot_dir: &Path,
) -> RestoreResultT<Option<UndoSnapshotInfo>> {
    // If current save doesn't exist, return Ok(None) - nothing to snapshot
    if !save_path.exists() {
        return Ok(None);
    }

    if !save_path.is_dir() {
        return Err(RestoreError::SaveNotFound(
            save_path.to_string_lossy().to_string(),
        ));
    }

    // Create undo snapshot directory if it doesn't exist
    if !undo_snapshot_dir.exists() {
        fs::create_dir_all(undo_snapshot_dir)
            .map_err(|e| FileOpsError::Io(e))?;
    }

    // Generate snapshot name and path
    let snapshot_name = generate_undo_snapshot_name();
    let snapshot_path = undo_snapshot_dir.join(&snapshot_name);

    // Delete existing snapshot if it exists (same timestamp scenario)
    if snapshot_path.exists() {
        delete_dir_recursive(&snapshot_path)?;
    }

    // Copy current save to snapshot location
    copy_dir_recursive(save_path, &snapshot_path)?;

    // Get snapshot metadata
    let size_bytes = crate::file_ops::get_dir_size(&snapshot_path)?;
    let size_formatted = crate::file_ops::format_size(size_bytes);

    let metadata = fs::metadata(&snapshot_path)
        .map_err(|e| FileOpsError::Io(e))?;
    let created = metadata
        .created()
        .or_else(|_| metadata.modified())
        .unwrap_or_else(|_| std::time::SystemTime::now());
    let created_dt: chrono::DateTime<chrono::Utc> = created.into();
    let created_at = created_dt.to_rfc3339();

    let save_name = save_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(Some(UndoSnapshotInfo {
        name: snapshot_name,
        path: snapshot_path.to_string_lossy().to_string(),
        size_bytes,
        size_formatted,
        created_at,
        save_name,
    }))
}

/// Restores a backup to the save directory with undo snapshot creation.
///
/// # Arguments
/// * `save_name` - Name of the save to restore
/// * `backup_name` - Name of the backup to restore
///
/// # Returns
/// `RestoreResultT<RestoreResult>` - Information about the restore operation
///
/// # Behavior
/// 1. Validates the backup exists
/// 2. Creates an "Undo snapshot" of the current save state (if it exists)
/// 3. Clears the current save directory
/// 4. Copies the backup contents to the save directory
///
/// # Safety
/// - Creates undo snapshot before any destructive operations
/// - If current save doesn't exist, proceeds without snapshot (first-time restore scenario)
pub fn restore_backup(save_name: &str, backup_name: &str) -> RestoreResultT<RestoreResult> {
    let config = config_module::load_config()?;
    let save_path = config.get_save_path()?;
    let backup_base_path = config.get_backup_path()?;

    let save_dir = save_path.join(save_name);
    let backup_save_dir = get_save_backup_dir(&backup_base_path, save_name);
    let backup_dir = backup_save_dir.join(backup_name);

    // Validate backup exists
    if !backup_dir.exists() {
        return Err(RestoreError::BackupNotFound(
            backup_dir.to_string_lossy().to_string(),
        ));
    }
    if !backup_dir.is_dir() {
        return Err(RestoreError::BackupNotFound(format!(
            "{} is not a directory",
            backup_dir.display()
        )));
    }

    // Create undo snapshot of current save (if it exists)
    let undo_snapshot_dir = get_undo_snapshot_dir(&backup_base_path, save_name);
    let undo_snapshot = create_undo_snapshot(&save_dir, &undo_snapshot_dir)?;

    // Clear current save directory if it exists
    if save_dir.exists() {
        delete_dir_recursive(&save_dir)?;
    }

    // Restore from backup (copy_dir_recursive creates the destination directory)
    copy_dir_recursive(&backup_dir, &save_dir)?;

    Ok(RestoreResult {
        save_path: save_dir.to_string_lossy().to_string(),
        save_name: save_name.to_string(),
        backup_path: backup_dir.to_string_lossy().to_string(),
        backup_name: backup_name.to_string(),
        undo_snapshot_path: undo_snapshot.as_ref().map(|u| u.path.clone()),
        has_undo_snapshot: undo_snapshot.is_some(),
    })
}

/// Lists all undo snapshots for a specific save.
///
/// # Arguments
/// * `save_name` - Name of the save
///
/// # Returns
/// `RestoreResultT<Vec<UndoSnapshotInfo>>` - List of undo snapshots sorted by creation time (newest first)
pub fn list_undo_snapshots(save_name: &str) -> RestoreResultT<Vec<UndoSnapshotInfo>> {
    let config = config_module::load_config()?;
    let backup_base_path = config.get_backup_path()?;
    let undo_snapshot_dir = get_undo_snapshot_dir(&backup_base_path, save_name);

    if !undo_snapshot_dir.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();

    for entry in fs::read_dir(&undo_snapshot_dir).map_err(|e| FileOpsError::Io(e))? {
        let entry = entry.map_err(|e| FileOpsError::Io(e))?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name_str) = name.to_str() {
                    let size_bytes = crate::file_ops::get_dir_size(&path)?;
                    let size_formatted = crate::file_ops::format_size(size_bytes);

                    let metadata = entry.metadata().map_err(|e| FileOpsError::Io(e))?;
                    let created = metadata
                        .created()
                        .or_else(|_| metadata.modified())
                        .unwrap_or_else(|_| std::time::SystemTime::now());
                    let created_dt: chrono::DateTime<chrono::Utc> = created.into();
                    let created_at = created_dt.to_rfc3339();

                    snapshots.push(UndoSnapshotInfo {
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
    snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(snapshots)
}

/// Restores from an undo snapshot.
///
/// # Arguments
/// * `save_name` - Name of the save
/// * `snapshot_name` - Name of the undo snapshot to restore from
///
/// # Returns
/// `RestoreResultT<RestoreResult>` - Information about the restore operation
///
/// # Behavior
/// 1. Validates the undo snapshot exists
/// 2. Clears the current save directory
/// 3. Copies the snapshot contents to the save directory
pub fn restore_from_undo_snapshot(
    save_name: &str,
    snapshot_name: &str,
) -> RestoreResultT<RestoreResult> {
    let config = config_module::load_config()?;
    let save_path = config.get_save_path()?;
    let backup_base_path = config.get_backup_path()?;

    let save_dir = save_path.join(save_name);
    let undo_snapshot_dir = get_undo_snapshot_dir(&backup_base_path, save_name);
    let snapshot_path = undo_snapshot_dir.join(snapshot_name);

    // Validate snapshot exists
    if !snapshot_path.exists() {
        return Err(RestoreError::BackupNotFound(
            snapshot_path.to_string_lossy().to_string(),
        ));
    }
    if !snapshot_path.is_dir() {
        return Err(RestoreError::BackupNotFound(format!(
            "{} is not a directory",
            snapshot_path.display()
        )));
    }

    // Clear current save directory if it exists
    if save_dir.exists() {
        delete_dir_recursive(&save_dir)?;
    }

    // Restore from snapshot (copy_dir_recursive creates the destination directory)
    copy_dir_recursive(&snapshot_path, &save_dir)?;

    Ok(RestoreResult {
        save_path: save_dir.to_string_lossy().to_string(),
        save_name: save_name.to_string(),
        backup_path: snapshot_path.to_string_lossy().to_string(),
        backup_name: snapshot_name.to_string(),
        undo_snapshot_path: None,
        has_undo_snapshot: false,
    })
}

/// Deletes an undo snapshot.
///
/// # Arguments
/// * `save_name` - Name of the save
/// * `snapshot_name` - Name of the undo snapshot to delete
///
/// # Returns
/// `RestoreResultT<()>` - Ok(()) on success
pub fn delete_undo_snapshot(save_name: &str, snapshot_name: &str) -> RestoreResultT<()> {
    let config = config_module::load_config()?;
    let backup_base_path = config.get_backup_path()?;

    let undo_snapshot_dir = get_undo_snapshot_dir(&backup_base_path, save_name);
    let snapshot_path = undo_snapshot_dir.join(snapshot_name);

    if !snapshot_path.exists() {
        return Err(RestoreError::BackupNotFound(
            snapshot_path.to_string_lossy().to_string(),
        ));
    }

    delete_dir_recursive(&snapshot_path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::{create_backup, generate_backup_name};
    use crate::config as config_module;
    use crate::config::Config;
    use serial_test::serial;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    /// Helper to create a test save directory with files
    fn create_test_save(save_dir: &Path) {
        fs::create_dir_all(save_dir.join("map")).unwrap();
        File::create(save_dir.join("save.bin"))
            .unwrap()
            .write_all(b"game state")
            .unwrap();
        File::create(save_dir.join("map/pchunk_0_0.dat"))
            .unwrap()
            .write_all(b"map data")
            .unwrap();
    }

    /// Helper to setup test config
    fn setup_test_config(save_dir: &Path, backup_dir: &Path) {
        let config = Config::with_paths(
            save_dir.to_str().unwrap().to_string(),
            backup_dir.to_str().unwrap().to_string(),
        );
        config_module::save_config(&config).unwrap();
    }

    /// Helper to modify save content
    fn modify_save_content(save_dir: &Path, content: &str) {
        let save_bin = save_dir.join("save.bin");
        fs::write(&save_bin, content).unwrap();
    }

    /// Helper to read save content
    fn read_save_content(save_dir: &Path) -> String {
        let save_bin = save_dir.join("save.bin");
        fs::read_to_string(&save_bin).unwrap()
    }

    #[test]
    fn test_generate_undo_snapshot_name_format() {
        let name = generate_undo_snapshot_name();
        // Format: undo_{YYYY-MM-DD}_{HH-mm-ss}
        assert!(name.starts_with("undo_"));
        let parts: Vec<&str> = name.split('_').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[1].chars().filter(|&c| c == '-').count() == 2); // Date has 2 dashes
        assert!(parts[2].chars().filter(|&c| c == '-').count() == 2); // Time has 2 dashes
    }

    #[test]
    fn test_get_undo_snapshot_dir() {
        let base = Path::new("/backups");
        let undo_dir = get_undo_snapshot_dir(base, "Survival");
        assert_eq!(undo_dir, Path::new("/backups/Survival_undo"));
    }

    #[test]
    fn test_create_undo_snapshot_when_save_exists() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        let undo_snapshot_dir = backup_base.path().join("Survival_undo");
        let snapshot = create_undo_snapshot(&save_dir, &undo_snapshot_dir).unwrap();

        assert!(snapshot.is_some());
        let snapshot_info = snapshot.unwrap();
        assert!(snapshot_info.name.starts_with("undo_"));
        assert!(snapshot_info.size_bytes > 0);
        assert!(!snapshot_info.size_formatted.is_empty());
    }

    #[test]
    fn test_create_undo_snapshot_when_save_not_exists() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        let undo_snapshot_dir = backup_base.path().join("Survival_undo");

        let snapshot = create_undo_snapshot(&save_dir, &undo_snapshot_dir).unwrap();

        assert!(snapshot.is_none());
    }

    #[test]
    #[serial]
    fn test_restore_backup_creates_undo_snapshot() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);
        let original_content = read_save_content(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        // Create a backup
        let backup_result = create_backup("Survival").unwrap();
        let backup_name = backup_result.backup_name;

        // Modify the save
        modify_save_content(&save_dir, "modified game state");
        assert_ne!(read_save_content(&save_dir), original_content);

        // Restore from backup
        let restore_result = restore_backup("Survival", &backup_name).unwrap();

        assert_eq!(restore_result.save_name, "Survival");
        assert_eq!(restore_result.backup_name, backup_name);
        assert!(restore_result.has_undo_snapshot);
        assert!(restore_result.undo_snapshot_path.is_some());

        // Verify save was restored
        assert_eq!(read_save_content(&save_dir), original_content);

        // Verify undo snapshot exists
        let undo_path = restore_result.undo_snapshot_path.unwrap();
        let undo_dir = Path::new(&undo_path);
        assert!(undo_dir.exists());
        let undo_content = read_save_content(undo_dir);
        assert_eq!(undo_content, "modified game state");
    }

    #[test]
    #[serial]
    fn test_restore_backup_when_save_not_exists() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        // Create a backup
        let backup_result = create_backup("Survival").unwrap();
        let backup_name = backup_result.backup_name;

        // Delete the save
        delete_dir_recursive(&save_dir).unwrap();
        assert!(!save_dir.exists());

        // Restore from backup (should work without undo snapshot)
        let restore_result = restore_backup("Survival", &backup_name).unwrap();

        assert_eq!(restore_result.save_name, "Survival");
        assert!(!restore_result.has_undo_snapshot);
        assert!(restore_result.undo_snapshot_path.is_none());

        // Verify save was restored
        assert!(save_dir.exists());
    }

    #[test]
    #[serial]
    fn test_restore_backup_not_found() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let result = restore_backup("Survival", "NonExistent");
        assert!(matches!(result, Err(RestoreError::BackupNotFound(_))));
    }

    #[test]
    #[serial]
    fn test_list_undo_snapshots() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        // Create a backup and restore to create undo snapshot
        let backup_result = create_backup("Survival").unwrap();
        modify_save_content(&save_dir, "modified");
        restore_backup("Survival", &backup_result.backup_name).unwrap();

        // Add delay for different timestamp
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Another restore to create second snapshot
        let backup_result2 = create_backup("Survival").unwrap();
        modify_save_content(&save_dir, "modified2");
        restore_backup("Survival", &backup_result2.backup_name).unwrap();

        let snapshots = list_undo_snapshots("Survival").unwrap();
        assert_eq!(snapshots.len(), 2);
        assert!(snapshots[0].name.starts_with("undo_"));
        assert!(snapshots[1].name.starts_with("undo_"));
    }

    #[test]
    #[serial]
    fn test_list_undo_snapshots_empty() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let snapshots = list_undo_snapshots("Survival").unwrap();
        assert_eq!(snapshots.len(), 0);
    }

    #[test]
    #[serial]
    fn test_restore_from_undo_snapshot() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);
        let _original_content = read_save_content(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        // Create backup, modify, and restore to create undo snapshot
        let backup_result = create_backup("Survival").unwrap();
        modify_save_content(&save_dir, "modified state");
        let restore_result = restore_backup("Survival", &backup_result.backup_name).unwrap();

        // Modify again
        modify_save_content(&save_dir, "another modification");
        assert_eq!(read_save_content(&save_dir), "another modification");

        // Get snapshot name from path
        let undo_path = restore_result.undo_snapshot_path.unwrap();
        let undo_path_buf = Path::new(&undo_path);
        let snapshot_name = undo_path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap();

        // Restore from undo snapshot
        let undo_restore_result = restore_from_undo_snapshot("Survival", snapshot_name).unwrap();

        assert_eq!(undo_restore_result.save_name, "Survival");
        assert!(!undo_restore_result.has_undo_snapshot);

        // Verify we got back the "modified state" content
        assert_eq!(read_save_content(&save_dir), "modified state");
    }

    #[test]
    #[serial]
    fn test_delete_undo_snapshot() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        // Create backup and restore to create undo snapshot
        let backup_result = create_backup("Survival").unwrap();
        let restore_result = restore_backup("Survival", &backup_result.backup_name).unwrap();

        let undo_path = restore_result.undo_snapshot_path.unwrap();
        let undo_path_buf = Path::new(&undo_path);
        let snapshot_name = undo_path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap();

        // Verify snapshot exists
        assert!(undo_path_buf.exists());

        // Delete snapshot
        delete_undo_snapshot("Survival", snapshot_name).unwrap();

        // Verify snapshot is deleted
        assert!(!undo_path_buf.exists());

        // Verify it's no longer in the list
        let snapshots = list_undo_snapshots("Survival").unwrap();
        assert_eq!(snapshots.len(), 0);
    }

    #[test]
    #[serial]
    fn test_delete_undo_snapshot_not_found() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let result = delete_undo_snapshot("Survival", "NonExistent");
        assert!(matches!(result, Err(RestoreError::BackupNotFound(_))));
    }

    #[test]
    #[serial]
    fn test_restore_from_undo_snapshot_not_found() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let result = restore_from_undo_snapshot("Survival", "NonExistent");
        assert!(matches!(result, Err(RestoreError::BackupNotFound(_))));
    }

    #[test]
    fn test_restore_result_serialization() {
        let result = RestoreResult {
            save_path: "/saves/Survival".to_string(),
            save_name: "Survival".to_string(),
            backup_path: "/backups/Survival/Survival_2024-12-28_10-00-00".to_string(),
            backup_name: "Survival_2024-12-28_10-00-00".to_string(),
            undo_snapshot_path: Some("/backups/Survival_undo/undo_2024-12-28_10-05-00".to_string()),
            has_undo_snapshot: true,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: RestoreResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.save_name, "Survival");
        assert!(parsed.has_undo_snapshot);
        assert!(parsed.undo_snapshot_path.is_some());
    }

    #[test]
    fn test_undo_snapshot_info_serialization() {
        let info = UndoSnapshotInfo {
            name: "undo_2024-12-28_10-00-00".to_string(),
            path: "/backups/Survival_undo/undo_2024-12-28_10-00-00".to_string(),
            size_bytes: 2048,
            size_formatted: "2.00 KB".to_string(),
            created_at: "2024-12-28T10:00:00Z".to_string(),
            save_name: "Survival".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: UndoSnapshotInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "undo_2024-12-28_10-00-00");
        assert_eq!(parsed.size_bytes, 2048);
        assert_eq!(parsed.save_name, "Survival");
    }

    #[test]
    fn test_restore_error_display() {
        let err = RestoreError::SaveNotFound("TestSave".to_string());
        assert_eq!(err.to_string(), "Save directory not found: TestSave");

        let err2 = RestoreError::BackupNotFound("test/path".to_string());
        assert_eq!(err2.to_string(), "Backup not found: test/path");

        let err3 = RestoreError::CurrentSaveNotFound("TestSave".to_string());
        assert_eq!(
            err3.to_string(),
            "Current save not found (nothing to snapshot): TestSave"
        );
    }

    #[test]
    #[serial]
    fn test_full_restore_cycle() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);
        let v1_content = read_save_content(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        // Create v1 backup
        let backup_v1 = create_backup("Survival").unwrap();

        // Modify to v2
        modify_save_content(&save_dir, "version 2");
        let v2_content = read_save_content(&save_dir);

        // Create v2 backup
        std::thread::sleep(std::time::Duration::from_secs(1));
        let backup_v2 = create_backup("Survival").unwrap();

        // Modify to v3
        modify_save_content(&save_dir, "version 3");

        // Restore v2 (should create undo snapshot of v3)
        let restore_v2 = restore_backup("Survival", &backup_v2.backup_name).unwrap();
        assert_eq!(read_save_content(&save_dir), v2_content);

        // Get undo snapshot name
        let undo_path = restore_v2.undo_snapshot_path.unwrap();
        let undo_path_buf = Path::new(&undo_path);
        let snapshot_name = undo_path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap();

        // Restore from undo snapshot to get back v3
        restore_from_undo_snapshot("Survival", snapshot_name).unwrap();
        assert_eq!(read_save_content(&save_dir), "version 3");

        // Restore v1
        restore_backup("Survival", &backup_v1.backup_name).unwrap();
        assert_eq!(read_save_content(&save_dir), v1_content);
    }
}
