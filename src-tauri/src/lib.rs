// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod auto_backup;
pub mod backup;
pub mod config;
pub mod file_ops;
pub mod restore;

use auto_backup::{AutoBackupStatus, AutoBackupResultT};
use backup::{BackupInfo, BackupResult, BackupResultT};
use config::{Config, ConfigResult, SaveEntry};
use file_ops::FileOpsResult;
use restore::{RestoreResult, RestoreResultT, UndoSnapshotInfo};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of directory size query
#[derive(Debug, Serialize, Deserialize)]
struct DirSizeResult {
    path: String,
    bytes: u64,
    formatted: String,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

/// Greet command - kept for testing from the original template
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Tauri command: Recursively copies a directory.
///
/// # Arguments
/// * `src_path` - Source directory path (as string)
/// * `dst_path` - Destination directory path (as string)
///
/// # Returns
/// `Result<(), FileOpsError>` - Ok(()) on success, Err with message on failure
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('copy_dir_recursive', {
///     srcPath: '/path/to/source',
///     dstPath: '/path/to/destination'
///   });
/// } catch (err) {
///   console.error('Copy failed:', err);
/// }
/// ```
#[tauri::command]
fn copy_dir_recursive(src_path: String, dst_path: String) -> FileOpsResult<()> {
    file_ops::copy_dir_recursive(Path::new(&src_path), Path::new(&dst_path))
}

/// Tauri command: Recursively deletes a directory.
///
/// # Arguments
/// * `path` - Path to directory to delete (as string)
///
/// # Returns
/// `Result<(), FileOpsError>` - Ok(()) on success, Err with message on failure
///
/// # Safety
/// This is a destructive operation. Frontend should confirm with user before calling.
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('delete_dir_recursive', {
///     path: '/path/to/delete'
///   });
/// } catch (err) {
///   console.error('Delete failed:', err);
/// }
/// ```
#[tauri::command]
fn delete_dir_recursive(path: String) -> FileOpsResult<()> {
    file_ops::delete_dir_recursive(Path::new(&path))
}

/// Tauri command: Calculates the total size of a directory.
///
/// # Arguments
/// * `path` - Path to directory (as string)
///
/// # Returns
/// `Result<DirSizeResult, FileOpsError>` - Size information on success, Err on failure
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const result = await invoke('get_dir_size', {
///   path: '/path/to/directory'
/// });
/// console.log(`Size: ${result.bytes} bytes (${result.formatted})`);
/// ```
#[tauri::command]
fn get_dir_size(path: String) -> FileOpsResult<DirSizeResult> {
    let bytes = file_ops::get_dir_size(Path::new(&path))?;
    let formatted = file_ops::format_size(bytes);
    Ok(DirSizeResult {
        path,
        bytes,
        formatted,
    })
}

/// Tauri command: Formats a byte count as human-readable string.
///
/// # Arguments
/// * `bytes` - Size in bytes
///
/// # Returns
/// Formatted string (e.g., "1.23 GB", "45.6 MB")
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const formatted = await invoke('format_size', { bytes: 1234567890 });
/// console.log(formatted); // "1.15 GB"
/// ```
#[tauri::command]
fn format_size(bytes: u64) -> String {
    file_ops::format_size(bytes)
}

/// Tauri command: Opens a path in the system file manager.
///
/// # Arguments
/// * `target_path` - Path to the file or directory (as string)
///
/// # Returns
/// `FileOpsResult<()>` - Ok(()) on success, Err on failure
///
/// # Behavior
/// - On macOS: Reveals the file/directory in Finder
/// - On Windows: Selects the file/directory in Explorer
/// - On Linux: Opens the parent directory in the default file manager
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// try {
///   await invoke('show_in_file_manager', {
///     targetPath: '/path/to/backup'
///   });
/// } catch (err) {
///   console.error('Failed to open:', err);
/// }
/// ```
#[tauri::command]
fn show_in_file_manager(target_path: String) -> FileOpsResult<()> {
    file_ops::show_in_file_manager(Path::new(&target_path))
}

// ============================================================================
// Config Commands (CORE-02)
// ============================================================================

/// Tauri command: Loads the application configuration.
///
/// # Returns
/// `ConfigResult<Config>` - Current configuration
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const config = await invoke('load_config');
/// console.log('Save path:', config.save_path);
/// console.log('Retention count:', config.retention_count);
/// ```
#[tauri::command]
fn load_config_command() -> ConfigResult<Config> {
    config::load_config()
}

/// Tauri command: Saves the application configuration.
///
/// # Arguments
/// * `config` - Configuration to save
///
/// # Returns
/// `ConfigResult<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('save_config', {
///   config: {
///     save_path: '/path/to/saves',
///     backup_path: '/path/to/backups',
///     retention_count: 15
///   }
/// });
/// ```
#[tauri::command]
fn save_config_command(config: Config) -> ConfigResult<()> {
    config::save_config(&config)
}

/// Tauri command: Updates the save path in the configuration.
///
/// # Arguments
/// * `savePath` - New save path (as string)
///
/// # Returns
/// `ConfigResult<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('update_save_path', {
///   savePath: '/custom/Zomboid/Saves'
/// });
/// ```
#[tauri::command]
fn update_save_path(save_path: String) -> ConfigResult<()> {
    config::update_save_path(save_path)
}

/// Tauri command: Updates the backup path in the configuration.
///
/// # Arguments
/// * `backupPath` - New backup path (as string)
///
/// # Returns
/// `ConfigResult<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('update_backup_path', {
///   backupPath: '/custom/ZomboidBackups'
/// });
/// ```
#[tauri::command]
fn update_backup_path(backup_path: String) -> ConfigResult<()> {
    config::update_backup_path(backup_path)
}

/// Tauri command: Updates the backup retention count.
///
/// # Arguments
/// * `count` - New retention count (must be >= 1)
///
/// # Returns
/// `ConfigResult<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('update_retention_count', { count: 20 });
/// ```
#[tauri::command]
fn update_retention_count(count: usize) -> ConfigResult<()> {
    config::update_retention_count(count)
}

/// Tauri command: Lists all save directories in the Zomboid saves folder.
///
/// # Deprecated
/// Consider using `list_save_entries` instead for full game mode support.
/// This command is kept for backward compatibility.
///
/// # Returns
/// `ConfigResult<Vec<String>>` - List of save names
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const saves = await invoke('list_save_directories');
/// console.log('Available saves:', saves);
/// // ["Survival", "Builder", "Adventure"]
/// ```
#[tauri::command]
fn list_save_directories() -> ConfigResult<Vec<String>> {
    config::list_save_directories()
}

/// Tauri command: Lists all save entries with game mode information.
///
/// # Returns
/// `ConfigResult<Vec<SaveEntry>>` - List of save entries with game mode info
///
/// # Behavior
/// Scans the Zomboid saves directory for the two-level structure `Saves/<GameMode>/<SaveName>`.
/// Also supports legacy flat structure for backwards compatibility.
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const entries = await invoke('list_save_entries');
/// console.log('Available saves:', entries);
/// // [
/// //   { gameMode: "Survival", saveName: "MySave1", relativePath: "Survival/MySave1" },
/// //   { gameMode: "Builder", saveName: "Builder1", relativePath: "Builder/Builder1" },
/// //   { gameMode: "", saveName: "OldSave", relativePath: "OldSave" }
/// // ]
/// ```
#[tauri::command]
fn list_save_entries() -> ConfigResult<Vec<SaveEntry>> {
    config::list_save_entries()
}

/// Tauri command: Lists save entries grouped by game mode.
///
/// # Returns
/// `ConfigResult<std::collections::HashMap<String, Vec<SaveEntry>>>` - Map of game mode to save entries
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const grouped = await invoke('list_save_entries_by_game_mode');
/// console.log('Saves by mode:', grouped);
/// // {
/// //   "Survival": [
/// //     { gameMode: "Survival", saveName: "MySave1", relativePath: "Survival/MySave1" }
/// //   ],
/// //   "Builder": [
/// //     { gameMode: "Builder", saveName: "Builder1", relativePath: "Builder/Builder1" }
/// //   ]
/// // }
/// ```
#[tauri::command]
fn list_save_entries_by_game_mode() -> ConfigResult<std::collections::HashMap<String, Vec<SaveEntry>>> {
    config::list_save_entries_by_game_mode()
}

/// Tauri command: Detects the default Zomboid save path for the current platform.
///
/// # Returns
/// `FileOpsResult<String>` - Detected save path
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const path = await invoke('detect_zomboid_save_path');
/// console.log('Auto-detected path:', path);
/// ```
#[tauri::command]
fn detect_zomboid_save_path() -> FileOpsResult<String> {
    let path = config::detect_zomboid_save_path()?;
    Ok(path.to_string_lossy().to_string())
}

// ============================================================================
// Backup Commands (CORE-03)
// ============================================================================

/// Tauri command: Creates a backup of the specified save directory (async).
///
/// # Arguments
/// * `saveName` - Name of the save to backup (must exist in save path)
///
/// # Returns
/// `BackupResultT<BackupResult>` - Information about the created backup
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const result = await invoke('create_backup', {
///   saveName: 'Survival'
/// });
/// console.log('Backup created:', result.backup_path);
/// console.log('Backups retained:', result.retained_count);
/// ```
#[tauri::command]
async fn create_backup_command(save_name: String) -> BackupResultT<BackupResult> {
    backup::create_backup_async(&save_name).await
}

/// Tauri command: Lists all backups for a specific save.
///
/// # Arguments
/// * `saveName` - Name of the save
///
/// # Returns
/// `BackupResultT<Vec<BackupInfo>>` - List of backups sorted by creation time (newest first)
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const backups = await invoke('list_backups', {
///   saveName: 'Survival'
/// });
/// backups.forEach(backup => {
///   console.log(`${backup.name}: ${backup.size_formatted}`);
/// });
/// ```
#[tauri::command]
fn list_backups_command(save_name: String) -> BackupResultT<Vec<BackupInfo>> {
    backup::list_backups(&save_name)
}

/// Tauri command: Gets detailed information about a specific backup.
///
/// # Arguments
/// * `saveName` - Name of the save
/// * `backupName` - Name of the backup directory
///
/// # Returns
/// `BackupResultT<BackupInfo>` - Detailed backup information
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const info = await invoke('get_backup_info', {
///   saveName: 'Survival',
///   backupName: 'Survival_2024-12-28_14-30-45'
/// });
/// console.log('Size:', info.size_formatted);
/// console.log('Created:', info.created_at);
/// ```
#[tauri::command]
fn get_backup_info_command(save_name: String, backup_name: String) -> BackupResultT<BackupInfo> {
    backup::get_backup_info(&save_name, &backup_name)
}

/// Tauri command: Lists all saves that have at least one backup.
///
/// # Returns
/// `BackupResultT<Vec<String>>` - List of save names with backups
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const saves = await invoke('list_saves_with_backups');
/// console.log('Saves with backups:', saves);
/// ```
#[tauri::command]
fn list_saves_with_backups_command() -> BackupResultT<Vec<String>> {
    backup::list_saves_with_backups()
}

/// Tauri command: Counts the number of backups for a specific save.
///
/// # Arguments
/// * `saveName` - Name of the save
///
/// # Returns
/// `BackupResultT<usize>` - Number of backups
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const count = await invoke('count_backups', {
///   saveName: 'Survival'
/// });
/// console.log('Total backups:', count);
/// ```
#[tauri::command]
fn count_backups_command(save_name: String) -> BackupResultT<usize> {
    backup::count_backups(&save_name)
}

/// Tauri command: Generates a timestamped backup name (for preview/testing).
///
/// # Arguments
/// * `saveName` - Name of the save
///
/// # Returns
/// Generated backup name
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const name = await invoke('generate_backup_name', {
///   saveName: 'Survival'
/// });
/// console.log('Generated name:', name);
/// // Output: "Survival_2024-12-28_14-30-45"
/// ```
#[tauri::command]
fn generate_backup_name_command(save_name: String) -> String {
    backup::generate_backup_name(&save_name)
}

/// Tauri command: Deletes a specific backup (async).
///
/// # Arguments
/// * `saveName` - Name of the save
/// * `backupName` - Name of the backup directory to delete
///
/// # Returns
/// `BackupResultT<()>` - Ok(()) on success
///
/// # Safety
/// This is a destructive operation. Frontend should confirm with user before calling.
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('delete_backup', {
///   saveName: 'Survival',
///   backupName: 'Survival_2024-12-28_14-30-45'
/// });
/// ```
#[tauri::command]
async fn delete_backup_command(save_name: String, backup_name: String) -> BackupResultT<()> {
    backup::delete_backup_async(&save_name, &backup_name).await
}

// ============================================================================
// Config Commands (CORE-02)
// ============================================================================

/// Tauri command: Gets the default backup storage path.
///
/// # Returns
/// `FileOpsResult<String>` - Default backup path
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const path = await invoke('get_default_backup_path');
/// console.log('Default backup path:', path);
/// ```
#[tauri::command]
fn get_default_backup_path() -> FileOpsResult<String> {
    let path = config::get_default_backup_path()?;
    Ok(path.to_string_lossy().to_string())
}

// ============================================================================
// Restore Commands (CORE-04)
// ============================================================================

/// Tauri command: Restores a backup with automatic undo snapshot creation (async).
///
/// # Arguments
/// * `saveName` - Name of the save to restore
/// * `backupName` - Name of the backup to restore
///
/// # Returns
/// `RestoreResultT<RestoreResult>` - Information about the restore operation
///
/// # Safety
/// This command automatically creates an "undo snapshot" of the current save state
/// before performing the restore. If the current save doesn't exist, the restore
/// proceeds without creating a snapshot (first-time restore scenario).
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const result = await invoke('restore_backup', {
///   saveName: 'Survival',
///   backupName: 'Survival_2024-12-28_14-30-45'
/// });
/// console.log('Restored to:', result.save_path);
/// console.log('Undo snapshot created:', result.has_undo_snapshot);
/// ```
#[tauri::command]
async fn restore_backup_command(save_name: String, backup_name: String) -> RestoreResultT<RestoreResult> {
    restore::restore_backup_async(&save_name, &backup_name).await
}

/// Tauri command: Lists all undo snapshots for a specific save.
///
/// # Arguments
/// * `saveName` - Name of the save
///
/// # Returns
/// `RestoreResultT<Vec<UndoSnapshotInfo>>` - List of undo snapshots sorted by creation time (newest first)
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const snapshots = await invoke('list_undo_snapshots', {
///   saveName: 'Survival'
/// });
/// snapshots.forEach(snapshot => {
///   console.log(`${snapshot.name}: ${snapshot.size_formatted}`);
/// });
/// ```
#[tauri::command]
fn list_undo_snapshots_command(save_name: String) -> RestoreResultT<Vec<UndoSnapshotInfo>> {
    restore::list_undo_snapshots(&save_name)
}

/// Tauri command: Restores from an undo snapshot.
///
/// # Arguments
/// * `saveName` - Name of the save
/// * `snapshotName` - Name of the undo snapshot to restore from
///
/// # Returns
/// `RestoreResultT<RestoreResult>` - Information about the restore operation
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const result = await invoke('restore_from_undo_snapshot', {
///   saveName: 'Survival',
///   snapshotName: 'undo_2024-12-28_14-30-45'
/// });
/// console.log('Restored from snapshot to:', result.save_path);
/// ```
#[tauri::command]
async fn restore_from_undo_snapshot_command(
    save_name: String,
    snapshot_name: String,
) -> RestoreResultT<RestoreResult> {
    restore::restore_from_undo_snapshot_async(&save_name, &snapshot_name).await
}

/// Tauri command: Deletes an undo snapshot (async).
///
/// # Arguments
/// * `saveName` - Name of the save
/// * `snapshotName` - Name of the undo snapshot to delete
///
/// # Returns
/// `RestoreResultT<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('delete_undo_snapshot', {
///   saveName: 'Survival',
///   snapshotName: 'undo_2024-12-28_14-30-45'
/// });
/// ```
#[tauri::command]
async fn delete_undo_snapshot_command(save_name: String, snapshot_name: String) -> RestoreResultT<()> {
    restore::delete_undo_snapshot_async(&save_name, &snapshot_name).await
}

// ============================================================================
// Auto Backup Commands (CORE-05)
// ============================================================================

/// Tauri command: Starts the auto backup service.
///
/// # Returns
/// `AutoBackupResultT<()>` - Ok(()) on success
///
/// # Behavior
/// - If already running, returns error
/// - Starts a background task that periodically creates backups for enabled saves
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('start_auto_backup');
/// ```
#[tauri::command]
async fn start_auto_backup_command() -> AutoBackupResultT<()> {
    auto_backup::start_auto_backup().await
}

/// Tauri command: Stops the auto backup service.
///
/// # Returns
/// `AutoBackupResultT<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('stop_auto_backup');
/// ```
#[tauri::command]
async fn stop_auto_backup_command() -> AutoBackupResultT<()> {
    auto_backup::stop_auto_backup().await
}

/// Tauri command: Gets the current auto backup status.
///
/// # Returns
/// `AutoBackupStatus` - Current status including running state, interval, and per-save states
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// const status = await invoke('get_auto_backup_status');
/// console.log('Running:', status.is_running);
/// console.log('Interval:', status.interval_seconds);
/// Object.entries(status.saves).forEach(([name, state]) => {
///   console.log(`${name}: enabled=${state.enabled}`);
/// });
/// ```
#[tauri::command]
async fn get_auto_backup_status_command() -> AutoBackupStatus {
    auto_backup::get_auto_backup_status().await
}

/// Tauri command: Sets the auto backup interval.
///
/// # Arguments
/// * `seconds` - Interval in seconds (must be between 60 and 86400)
///
/// # Returns
/// `AutoBackupResultT<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// // Set interval to 10 minutes (600 seconds)
/// await invoke('set_auto_backup_interval', { seconds: 600 });
/// ```
#[tauri::command]
async fn set_auto_backup_interval_command(seconds: u64) -> AutoBackupResultT<()> {
    auto_backup::set_auto_backup_interval(seconds).await
}

/// Tauri command: Enables auto backup for a specific save.
///
/// # Arguments
/// * `saveName` - Name of the save
///
/// # Returns
/// `AutoBackupResultT<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('enable_auto_backup', {
///   saveName: 'Survival'
/// });
/// ```
#[tauri::command]
async fn enable_auto_backup_command(save_name: String) -> AutoBackupResultT<()> {
    auto_backup::enable_auto_backup(save_name).await
}

/// Tauri command: Disables auto backup for a specific save.
///
/// # Arguments
/// * `saveName` - Name of the save
///
/// # Returns
/// `AutoBackupResultT<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('disable_auto_backup', {
///   saveName: 'Survival'
/// });
/// ```
#[tauri::command]
async fn disable_auto_backup_command(save_name: String) -> AutoBackupResultT<()> {
    auto_backup::disable_auto_backup(save_name).await
}

/// Tauri command: Refreshes the auto backup save states from available saves.
///
/// # Returns
/// `AutoBackupResultT<()>` - Ok(()) on success
///
/// # Example (Frontend)
/// ```javascript
/// import { invoke } from '@tauri-apps/api/core';
///
/// await invoke('refresh_auto_backup_saves');
/// ```
#[tauri::command]
async fn refresh_auto_backup_saves_command() -> AutoBackupResultT<()> {
    auto_backup::refresh_auto_backup_saves().await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            copy_dir_recursive,
            delete_dir_recursive,
            get_dir_size,
            format_size,
            show_in_file_manager,
            // Config commands (CORE-02)
            load_config_command,
            save_config_command,
            update_save_path,
            update_backup_path,
            update_retention_count,
            list_save_directories,
            list_save_entries,
            list_save_entries_by_game_mode,
            detect_zomboid_save_path,
            get_default_backup_path,
            // Backup commands (CORE-03)
            create_backup_command,
            list_backups_command,
            get_backup_info_command,
            list_saves_with_backups_command,
            count_backups_command,
            generate_backup_name_command,
            delete_backup_command,
            // Restore commands (CORE-04)
            restore_backup_command,
            list_undo_snapshots_command,
            restore_from_undo_snapshot_command,
            delete_undo_snapshot_command,
            // Auto backup commands (CORE-05)
            start_auto_backup_command,
            stop_auto_backup_command,
            get_auto_backup_status_command,
            set_auto_backup_interval_command,
            enable_auto_backup_command,
            disable_auto_backup_command,
            refresh_auto_backup_saves_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
