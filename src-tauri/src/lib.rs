// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod file_ops;

use config::{Config, ConfigResult};
use file_ops::FileOpsResult;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            copy_dir_recursive,
            delete_dir_recursive,
            get_dir_size,
            format_size,
            // Config commands (CORE-02)
            load_config_command,
            save_config_command,
            update_save_path,
            update_backup_path,
            update_retention_count,
            list_save_directories,
            detect_zomboid_save_path,
            get_default_backup_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
