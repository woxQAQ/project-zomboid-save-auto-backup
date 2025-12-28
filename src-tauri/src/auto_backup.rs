//! Auto backup background task for Project Zomboid save backup/restore.
//!
//! This module provides:
//! - Background timer-based automatic backup functionality
//! - State management for auto backup on/off status
//! - Configurable backup intervals
//! - Per-save auto backup enable/disable

use crate::backup::{BackupError, BackupResult};
use crate::config::ConfigError;
use crate::config as config_module;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::Instant;

/// Default auto backup interval in seconds (5 minutes).
pub const DEFAULT_AUTO_BACKUP_INTERVAL: u64 = 300;

/// Minimum auto backup interval in seconds (1 minute).
pub const MIN_AUTO_BACKUP_INTERVAL: u64 = 60;

/// Maximum auto backup interval in seconds (24 hours).
pub const MAX_AUTO_BACKUP_INTERVAL: u64 = 86400;

/// Auto backup state for a single save.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveAutoBackupState {
    /// Name of the save
    pub save_name: String,
    /// Whether auto backup is enabled for this save
    pub enabled: bool,
    /// Last time a backup was created (ISO 8601 timestamp)
    pub last_backup_time: Option<String>,
    /// Next scheduled backup time (ISO 8601 timestamp)
    pub next_backup_time: Option<String>,
}

/// Overall auto backup status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoBackupStatus {
    /// Whether the auto backup service is running globally
    pub is_running: bool,
    /// Configured backup interval in seconds
    pub interval_seconds: u64,
    /// Per-save auto backup states
    pub saves: HashMap<String, SaveAutoBackupState>,
    /// Timestamp when the service was started (ISO 8601)
    pub started_at: Option<String>,
}

/// Error type for auto backup operations.
#[derive(Debug)]
pub enum AutoBackupError {
    /// File operation error
    FileOp(crate::file_ops::FileOpsError),
    /// Config error
    Config(ConfigError),
    /// Backup error
    Backup(BackupError),
    /// Auto backup is not running
    NotRunning,
    /// Auto backup is already running
    AlreadyRunning,
    /// Invalid interval
    InvalidInterval(String),
    /// Save not found
    SaveNotFound(String),
}

impl From<crate::file_ops::FileOpsError> for AutoBackupError {
    fn from(err: crate::file_ops::FileOpsError) -> Self {
        AutoBackupError::FileOp(err)
    }
}

impl From<ConfigError> for AutoBackupError {
    fn from(err: ConfigError) -> Self {
        AutoBackupError::Config(err)
    }
}

impl From<BackupError> for AutoBackupError {
    fn from(err: BackupError) -> Self {
        AutoBackupError::Backup(err)
    }
}

impl std::fmt::Display for AutoBackupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutoBackupError::FileOp(err) => write!(f, "File operation error: {}", err),
            AutoBackupError::Config(err) => write!(f, "Config error: {}", err),
            AutoBackupError::Backup(err) => write!(f, "Backup error: {}", err),
            AutoBackupError::NotRunning => write!(f, "Auto backup service is not running"),
            AutoBackupError::AlreadyRunning => write!(f, "Auto backup service is already running"),
            AutoBackupError::InvalidInterval(msg) => write!(f, "Invalid interval: {}", msg),
            AutoBackupError::SaveNotFound(name) => write!(f, "Save not found: {}", name),
        }
    }
}

impl std::error::Error for AutoBackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AutoBackupError::FileOp(err) => Some(err),
            AutoBackupError::Config(err) => Some(err),
            AutoBackupError::Backup(err) => Some(err),
            _ => None,
        }
    }
}

impl Serialize for AutoBackupError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Result type for auto backup operations.
pub type AutoBackupResultT<T> = Result<T, AutoBackupError>;

/// Global auto backup manager state.
#[derive(Clone)]
pub struct AutoBackupManager {
    inner: Arc<AutoBackupManagerInner>,
}

/// Inner state of the auto backup manager.
struct AutoBackupManagerInner {
    /// Whether the service is running
    is_running: RwLock<bool>,
    /// Auto backup interval in seconds
    interval: RwLock<u64>,
    /// Per-save enabled states
    save_states: RwLock<HashMap<String, SaveAutoBackupState>>,
    /// Start time
    started_at: RwLock<Option<String>>,
}

impl AutoBackupManager {
    /// Creates a new auto backup manager instance.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AutoBackupManagerInner {
                is_running: RwLock::new(false),
                interval: RwLock::new(DEFAULT_AUTO_BACKUP_INTERVAL),
                save_states: RwLock::new(HashMap::new()),
                started_at: RwLock::new(None),
            }),
        }
    }

    /// Starts the auto backup service.
    ///
    /// # Returns
    /// `AutoBackupResultT<()>` - Ok(()) on success
    ///
    /// # Behavior
    /// - If already running, returns AlreadyRunning error
    /// - Starts a background task that periodically creates backups for enabled saves
    pub async fn start(&self) -> AutoBackupResultT<()> {
        let mut is_running = self.inner.is_running.write().await;
        if *is_running {
            return Err(AutoBackupError::AlreadyRunning);
        }

        *is_running = true;

        // Set start time
        let mut started_at = self.inner.started_at.write().await;
        *started_at = Some(chrono::Utc::now().to_rfc3339());
        drop(started_at);

        // Spawn the background task
        let manager = self.clone();
        tokio::spawn(async move {
            manager.run_backup_loop().await;
        });

        Ok(())
    }

    /// Stops the auto backup service.
    ///
    /// # Returns
    /// `AutoBackupResultT<()>` - Ok(()) on success
    pub async fn stop(&self) -> AutoBackupResultT<()> {
        let mut is_running = self.inner.is_running.write().await;
        if !*is_running {
            return Err(AutoBackupError::NotRunning);
        }
        *is_running = false;

        // Clear start time
        let mut started_at = self.inner.started_at.write().await;
        *started_at = None;

        Ok(())
    }

    /// Checks if the auto backup service is running.
    pub async fn is_running(&self) -> bool {
        *self.inner.is_running.read().await
    }

    /// Sets the auto backup interval.
    ///
    /// # Arguments
    /// * `seconds` - Interval in seconds (must be between MIN and MAX)
    pub async fn set_interval(&self, seconds: u64) -> AutoBackupResultT<()> {
        if !(MIN_AUTO_BACKUP_INTERVAL..=MAX_AUTO_BACKUP_INTERVAL).contains(&seconds) {
            return Err(AutoBackupError::InvalidInterval(
                format!("Interval must be between {} and {} seconds", MIN_AUTO_BACKUP_INTERVAL, MAX_AUTO_BACKUP_INTERVAL)
            ));
        }

        let mut interval = self.inner.interval.write().await;
        *interval = seconds;
        Ok(())
    }

    /// Gets the current auto backup interval.
    pub async fn get_interval(&self) -> u64 {
        *self.inner.interval.read().await
    }

    /// Enables auto backup for a specific save.
    ///
    /// # Arguments
    /// * `save_name` - Name of the save
    pub async fn enable_save(&self, save_name: &str) -> AutoBackupResultT<()> {
        // Verify the save exists
        let config = config_module::load_config()?;
        let save_path = config.get_save_path()?;
        let save_dir = save_path.join(save_name);
        if !save_dir.exists() {
            return Err(AutoBackupError::SaveNotFound(save_name.to_string()));
        }

        let mut states = self.inner.save_states.write().await;
        let state = states.entry(save_name.to_string()).or_insert_with(|| {
            SaveAutoBackupState {
                save_name: save_name.to_string(),
                enabled: false,
                last_backup_time: None,
                next_backup_time: None,
            }
        });
        state.enabled = true;
        state.next_backup_time = Some(chrono::Utc::now().to_rfc3339());
        Ok(())
    }

    /// Disables auto backup for a specific save.
    ///
    /// # Arguments
    /// * `save_name` - Name of the save
    pub async fn disable_save(&self, save_name: &str) {
        let mut states = self.inner.save_states.write().await;
        if let Some(state) = states.get_mut(save_name) {
            state.enabled = false;
            state.next_backup_time = None;
        }
    }

    /// Checks if auto backup is enabled for a specific save.
    pub async fn is_save_enabled(&self, save_name: &str) -> bool {
        let states = self.inner.save_states.read().await;
        states.get(save_name).map(|s| s.enabled).unwrap_or(false)
    }

    /// Gets the current auto backup status.
    pub async fn get_status(&self) -> AutoBackupStatus {
        let is_running = *self.inner.is_running.read().await;
        let interval = *self.inner.interval.read().await;
        let started_at = self.inner.started_at.read().await.clone();
        let saves = self.inner.save_states.read().await.clone();

        AutoBackupStatus {
            is_running,
            interval_seconds: interval,
            saves,
            started_at,
        }
    }

    /// Main backup loop that runs in the background.
    async fn run_backup_loop(&self) {
        let mut last_backup_times: HashMap<String, Instant> = HashMap::new();

        loop {
            // Check if still running
            if !self.is_running().await {
                break;
            }

            // Get current interval
            let interval_secs = self.get_interval().await;
            let interval_duration = Duration::from_secs(interval_secs);

            // Get enabled saves
            let enabled_saves = {
                let states = self.inner.save_states.read().await;
                states
                    .iter()
                    .filter(|(_, state)| state.enabled)
                    .map(|(name, _)| name.clone())
                    .collect::<Vec<_>>()
            };

            // Process each enabled save
            for save_name in enabled_saves {
                let last_backup = last_backup_times.get(&save_name);

                // Check if enough time has passed since last backup
                if last_backup.is_none_or(|t| t.elapsed() >= interval_duration) {
                    // Perform backup
                    match self.backup_save(&save_name).await {
                        Ok(_) => {
                            last_backup_times.insert(save_name.clone(), Instant::now());

                            // Update state
                            let mut states = self.inner.save_states.write().await;
                            if let Some(state) = states.get_mut(&save_name) {
                                state.last_backup_time = Some(chrono::Utc::now().to_rfc3339());
                                // Calculate next backup time
                                let next = chrono::Utc::now() + chrono::Duration::seconds(interval_secs as i64);
                                state.next_backup_time = Some(next.to_rfc3339());
                            }
                        }
                        Err(e) => {
                            eprintln!("Auto backup failed for {}: {}", save_name, e);
                        }
                    }
                }
            }

            // Sleep for a short duration before checking again
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }

    /// Performs a backup for a specific save.
    async fn backup_save(&self, save_name: &str) -> AutoBackupResultT<BackupResult> {
        // Use tokio::task::spawn_blocking to run the synchronous backup operation
        let save_name = save_name.to_string();
        let result = tokio::task::spawn_blocking(move || {
            crate::backup::create_backup(&save_name)
        })
        .await
        .map_err(|e| AutoBackupError::Backup(BackupError::SaveNotFound(
            format!("Task join error: {}", e)
        )))??;

        Ok(result)
    }

    /// Refreshes save states from the current save directories.
    ///
    /// This should be called when the UI loads to sync with available saves.
    pub async fn refresh_save_states(&self) -> AutoBackupResultT<()> {
        let config = config_module::load_config()?;
        let save_path = config.get_save_path()?;

        if !save_path.exists() {
            return Ok(());
        }

        let mut states = self.inner.save_states.write().await;

        // Read save directories
        let mut new_states = HashMap::new();
        for entry in std::fs::read_dir(&save_path)
            .map_err(|e| AutoBackupError::FileOp(crate::file_ops::FileOpsError::Io(e)))?
        {
            let entry = entry.map_err(|e| AutoBackupError::FileOp(crate::file_ops::FileOpsError::Io(e)))?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        // Preserve existing state if available
                        let existing_state = states.get(name_str);
                        let (enabled, last_backup, next_backup) = existing_state.map_or((false, None, None), |s| {
                            (s.enabled, s.last_backup_time.clone(), s.next_backup_time.clone())
                        });

                        new_states.insert(name_str.to_string(), SaveAutoBackupState {
                            save_name: name_str.to_string(),
                            enabled,
                            last_backup_time: last_backup,
                            next_backup_time: next_backup,
                        });
                    }
                }
            }
        }

        *states = new_states;
        Ok(())
    }
}

impl Default for AutoBackupManager {
    fn default() -> Self {
        Self::new()
    }
}

// Global singleton instance
static GLOBAL_MANAGER: std::sync::OnceLock<AutoBackupManager> = std::sync::OnceLock::new();

/// Gets the global auto backup manager instance.
pub fn get_manager() -> &'static AutoBackupManager {
    GLOBAL_MANAGER.get_or_init(AutoBackupManager::new)
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Starts the auto backup service.
#[tauri::command]
pub async fn start_auto_backup() -> AutoBackupResultT<()> {
    get_manager().start().await
}

/// Stops the auto backup service.
#[tauri::command]
pub async fn stop_auto_backup() -> AutoBackupResultT<()> {
    get_manager().stop().await
}

/// Gets the current auto backup status.
#[tauri::command]
pub async fn get_auto_backup_status() -> AutoBackupStatus {
    get_manager().get_status().await
}

/// Sets the auto backup interval.
#[tauri::command]
pub async fn set_auto_backup_interval(seconds: u64) -> AutoBackupResultT<()> {
    get_manager().set_interval(seconds).await
}

/// Enables auto backup for a specific save.
#[tauri::command]
pub async fn enable_auto_backup(save_name: String) -> AutoBackupResultT<()> {
    get_manager().enable_save(&save_name).await
}

/// Disables auto backup for a specific save.
#[tauri::command]
pub async fn disable_auto_backup(save_name: String) -> AutoBackupResultT<()> {
    get_manager().disable_save(&save_name).await;
    Ok(())
}

/// Refreshes the auto backup save states from available saves.
#[tauri::command]
pub async fn refresh_auto_backup_saves() -> AutoBackupResultT<()> {
    get_manager().refresh_save_states().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use serial_test::serial;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    /// Helper to create a test save directory with files
    fn create_test_save(save_dir: &Path) {
        fs::create_dir_all(save_dir.join("map")).unwrap();
        File::create(save_dir.join("save.bin")).unwrap().write_all(b"game state").unwrap();
        File::create(save_dir.join("map/pchunk_0_0.dat")).unwrap().write_all(b"map data").unwrap();
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
    fn test_default_interval() {
        assert_eq!(DEFAULT_AUTO_BACKUP_INTERVAL, 300);
        assert_eq!(MIN_AUTO_BACKUP_INTERVAL, 60);
        assert_eq!(MAX_AUTO_BACKUP_INTERVAL, 86400);
    }

    #[test]
    fn test_auto_backup_manager_new() {
        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            assert!(!manager.is_running().await);
        });
    }

    #[test]
    fn test_auto_backup_manager_default() {
        let manager = AutoBackupManager::default();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            assert!(!manager.is_running().await);
        });
    }

    #[test]
    fn test_set_interval_valid() {
        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            assert!(manager.set_interval(120).await.is_ok());
            assert_eq!(manager.get_interval().await, 120);
        });
    }

    #[test]
    fn test_set_interval_too_small() {
        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let result = manager.set_interval(30).await;
            assert!(matches!(result, Err(AutoBackupError::InvalidInterval(_))));
        });
    }

    #[test]
    fn test_set_interval_too_large() {
        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let result = manager.set_interval(100000).await;
            assert!(matches!(result, Err(AutoBackupError::InvalidInterval(_))));
        });
    }

    #[test]
    #[serial]
    fn test_enable_save_success() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        let save_dir = save_base.path().join("Survival");
        create_test_save(&save_dir);

        setup_test_config(save_base.path(), backup_base.path());

        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            assert!(manager.enable_save("Survival").await.is_ok());
            assert!(manager.is_save_enabled("Survival").await);
            assert!(!manager.is_save_enabled("NonExistent").await);
        });
    }

    #[test]
    #[serial]
    fn test_enable_save_not_found() {
        let save_base = TempDir::new().unwrap();
        let backup_base = TempDir::new().unwrap();

        setup_test_config(save_base.path(), backup_base.path());

        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let result = manager.enable_save("NonExistent").await;
            assert!(matches!(result, Err(AutoBackupError::SaveNotFound(_))));
        });
    }

    #[test]
    fn test_disable_save() {
        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            // Disable non-existent save should not error
            manager.disable_save("NonExistent").await;

            // Note: We can't test enable/disable cycle here because we don't have
            // a valid save directory setup. The enable_save will fail with SaveNotFound.
            // This is expected behavior - saves must exist to be enabled.
        });
    }

    #[test]
    fn test_get_status() {
        let manager = AutoBackupManager::new();
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let status = manager.get_status().await;
            assert!(!status.is_running);
            assert_eq!(status.interval_seconds, DEFAULT_AUTO_BACKUP_INTERVAL);
            assert!(status.saves.is_empty());
            assert!(status.started_at.is_none());
        });
    }

    #[test]
    fn test_auto_backup_error_display() {
        let err = AutoBackupError::NotRunning;
        assert_eq!(err.to_string(), "Auto backup service is not running");

        let err2 = AutoBackupError::AlreadyRunning;
        assert_eq!(err2.to_string(), "Auto backup service is already running");

        let err3 = AutoBackupError::SaveNotFound("TestSave".to_string());
        assert_eq!(err3.to_string(), "Save not found: TestSave");
    }

    #[test]
    fn test_save_auto_backup_state_serialization() {
        let state = SaveAutoBackupState {
            save_name: "Survival".to_string(),
            enabled: true,
            last_backup_time: Some("2024-12-28T10:00:00Z".to_string()),
            next_backup_time: Some("2024-12-28T10:05:00Z".to_string()),
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: SaveAutoBackupState = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.save_name, "Survival");
        assert!(parsed.enabled);
        assert_eq!(parsed.last_backup_time, Some("2024-12-28T10:00:00Z".to_string()));
    }

    #[test]
    fn test_auto_backup_status_serialization() {
        let mut saves = HashMap::new();
        saves.insert("Survival".to_string(), SaveAutoBackupState {
            save_name: "Survival".to_string(),
            enabled: true,
            last_backup_time: None,
            next_backup_time: None,
        });

        let status = AutoBackupStatus {
            is_running: false,
            interval_seconds: 300,
            saves,
            started_at: Some("2024-12-28T10:00:00Z".to_string()),
        };

        let json = serde_json::to_string(&status).unwrap();
        let parsed: AutoBackupStatus = serde_json::from_str(&json).unwrap();

        assert!(!parsed.is_running);
        assert_eq!(parsed.interval_seconds, 300);
        assert!(parsed.saves.contains_key("Survival"));
    }

    #[test]
    fn test_get_manager_singleton() {
        let m1 = get_manager();
        let m2 = get_manager();
        // Should be the same instance
        assert!(Arc::ptr_eq(&m1.inner, &m2.inner));
    }
}
