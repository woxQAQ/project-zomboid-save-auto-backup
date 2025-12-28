import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Configuration interface matching the Rust Config struct
 */
interface Config {
  save_path: string | null;
  backup_path: string | null;
  retention_count: number;
}

interface SettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Settings modal component for configuring application paths and backup policy.
 *
 * Features:
 * - Save path selection with auto-detection fallback
 * - Backup path selection with default fallback
 * - Retention count configuration (1-100)
 * - Validation and error handling
 */
export const Settings: React.FC<SettingsProps> = ({ isOpen, onClose }) => {
  const [config, setConfig] = useState<Config>({
    save_path: null,
    backup_path: null,
    retention_count: 10,
  });
  const [savePathInput, setSavePathInput] = useState("");
  const [backupPathInput, setBackupPathInput] = useState("");
  const [retentionInput, setRetentionInput] = useState("10");
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Ref to track the timeout for cleanup
  const successTimeoutRef = useRef<number | null>(null);

  // Clear timeout on unmount to prevent race conditions
  useEffect(() => {
    return () => {
      if (successTimeoutRef.current !== null) {
        clearTimeout(successTimeoutRef.current);
      }
    };
  }, []);

  const loadConfig = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const loadedConfig = await invoke<Config>("load_config_command");
      setConfig(loadedConfig);
      setSavePathInput(loadedConfig.save_path || "");
      setBackupPathInput(loadedConfig.backup_path || "");
      setRetentionInput(loadedConfig.retention_count.toString());
    } catch (err) {
      setError(`Failed to load configuration: ${err}`);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Load configuration on mount
  useEffect(() => {
    if (isOpen) {
      loadConfig();
    }
  }, [isOpen, loadConfig]);

  const handleSelectSavePath = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Zomboid Saves Directory",
      });

      if (selected) {
        setSavePathInput(selected as string);
        setError(null);
      }
    } catch (err) {
      setError(`Failed to open folder dialog: ${err}`);
    }
  };

  const handleSelectBackupPath = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Backup Directory",
      });

      if (selected) {
        setBackupPathInput(selected as string);
        setError(null);
      }
    } catch (err) {
      setError(`Failed to open folder dialog: ${err}`);
    }
  };

  const handleAutoDetectSavePath = async () => {
    try {
      const detectedPath = await invoke<string>("detect_zomboid_save_path");
      setSavePathInput(detectedPath);
      setError(null);
    } catch (err) {
      setError(`Failed to auto-detect save path: ${err}`);
    }
  };

  const handleGetDefaultBackupPath = async () => {
    try {
      const defaultPath = await invoke<string>("get_default_backup_path");
      setBackupPathInput(defaultPath);
      setError(null);
    } catch (err) {
      setError(`Failed to get default backup path: ${err}`);
    }
  };

  const validateInputs = (): string | null => {
    const retention = parseInt(retentionInput, 10);
    if (Number.isNaN(retention) || retention < 1) {
      return "Retention count must be at least 1";
    }
    if (retention > 100) {
      return "Retention count cannot exceed 100";
    }
    return null;
  };

  const handleSave = async () => {
    const validationError = validateInputs();
    if (validationError) {
      setError(validationError);
      return;
    }

    setIsSaving(true);
    setError(null);
    setSuccessMessage(null);

    try {
      // Save the complete configuration
      const newConfig: Config = {
        save_path: savePathInput.trim() || null,
        backup_path: backupPathInput.trim() || null,
        retention_count: parseInt(retentionInput, 10),
      };

      await invoke("save_config_command", { config: newConfig });
      setConfig(newConfig);
      setSuccessMessage("Settings saved successfully!");

      // Auto-close after success
      // Clear any existing timeout first
      if (successTimeoutRef.current !== null) {
        clearTimeout(successTimeoutRef.current);
      }
      successTimeoutRef.current = window.setTimeout(() => {
        onClose();
        setSuccessMessage(null);
        successTimeoutRef.current = null;
      }, 1500);
    } catch (err) {
      setError(`Failed to save configuration: ${err}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleCancel = () => {
    // Reset to current config
    setSavePathInput(config.save_path || "");
    setBackupPathInput(config.backup_path || "");
    setRetentionInput(config.retention_count.toString());
    setError(null);
    setSuccessMessage(null);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-[#1a1a1a] border border-gray-800 rounded-lg shadow-xl w-full max-w-2xl mx-4 max-h-[90vh] overflow-auto">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-800">
          <h2 className="text-xl font-semibold text-foreground">Settings</h2>
          <button
            type="button"
            onClick={handleCancel}
            className="p-1 rounded-lg hover:bg-gray-800 transition-colors"
            aria-label="Close settings"
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="w-5 h-5 text-gray-400"
            >
              <title>Close</title>
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-6">
          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
            </div>
          ) : (
            <>
              {/* Error Message */}
              {error && (
                <div className="bg-red-900/20 border border-red-800 rounded-lg p-4">
                  <p className="text-sm text-red-400">{error}</p>
                </div>
              )}

              {/* Success Message */}
              {successMessage && (
                <div className="bg-green-900/20 border border-green-800 rounded-lg p-4">
                  <p className="text-sm text-green-400">{successMessage}</p>
                </div>
              )}

              {/* Save Path Section */}
              <div className="space-y-2">
                <label htmlFor="save-path" className="block text-sm font-medium text-foreground">
                  Zomboid Save Path
                  <span className="text-gray-500 font-normal ml-2">
                    (Directory containing save folders)
                  </span>
                </label>
                <div className="flex gap-2">
                  <input
                    id="save-path"
                    type="text"
                    value={savePathInput}
                    onChange={(e) => setSavePathInput(e.target.value)}
                    placeholder="Auto-detected from ~/Zomboid/Saves"
                    className="flex-1 bg-gray-900 border border-gray-800 rounded-lg px-4 py-2 text-foreground placeholder-gray-600 focus:outline-none focus:border-primary"
                  />
                  <button
                    type="button"
                    onClick={handleSelectSavePath}
                    className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-foreground rounded-lg transition-colors"
                  >
                    Browse
                  </button>
                  <button
                    type="button"
                    onClick={handleAutoDetectSavePath}
                    className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-primary rounded-lg transition-colors"
                    title="Auto-detect default Zomboid save path"
                  >
                    Auto
                  </button>
                </div>
                <p className="text-xs text-gray-500">
                  Leave empty to use auto-detection. Typical: ~/Zomboid/Saves
                </p>
              </div>

              {/* Backup Path Section */}
              <div className="space-y-2">
                <label htmlFor="backup-path" className="block text-sm font-medium text-foreground">
                  Backup Storage Path
                  <span className="text-gray-500 font-normal ml-2">(Where backups are stored)</span>
                </label>
                <div className="flex gap-2">
                  <input
                    id="backup-path"
                    type="text"
                    value={backupPathInput}
                    onChange={(e) => setBackupPathInput(e.target.value)}
                    placeholder="Default: ~/ZomboidBackups"
                    className="flex-1 bg-gray-900 border border-gray-800 rounded-lg px-4 py-2 text-foreground placeholder-gray-600 focus:outline-none focus:border-primary"
                  />
                  <button
                    type="button"
                    onClick={handleSelectBackupPath}
                    className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-foreground rounded-lg transition-colors"
                  >
                    Browse
                  </button>
                  <button
                    type="button"
                    onClick={handleGetDefaultBackupPath}
                    className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-primary rounded-lg transition-colors"
                    title="Use default backup path"
                  >
                    Default
                  </button>
                </div>
                <p className="text-xs text-gray-500">
                  Leave empty to use default location. Backups organized as:
                  BackupPath/SaveName/SaveName_YYYY-MM-DD_HH-mm-ss
                </p>
              </div>

              {/* Retention Count Section */}
              <div className="space-y-2">
                <label
                  htmlFor="retention-count"
                  className="block text-sm font-medium text-foreground"
                >
                  Backup Retention Count
                  <span className="text-gray-500 font-normal ml-2">
                    (Max backups to keep per save)
                  </span>
                </label>
                <div className="flex items-center gap-4">
                  <input
                    id="retention-count"
                    type="number"
                    min="1"
                    max="100"
                    value={retentionInput}
                    onChange={(e) => setRetentionInput(e.target.value)}
                    className="w-24 bg-gray-900 border border-gray-800 rounded-lg px-4 py-2 text-foreground focus:outline-none focus:border-primary"
                  />
                  <input
                    type="range"
                    min="1"
                    max="100"
                    value={Math.max(1, Math.min(100, parseInt(retentionInput, 10) || 10))}
                    onChange={(e) => setRetentionInput(e.target.value)}
                    className="flex-1 accent-primary"
                  />
                </div>
                <p className="text-xs text-gray-500">
                  Old backups exceeding this count will be automatically deleted. Recommended: 5-20
                </p>
              </div>

              {/* Info Box */}
              <div className="bg-gray-900/50 border border-gray-800 rounded-lg p-4">
                <h3 className="text-sm font-medium text-foreground mb-2">Path Information</h3>
                <ul className="text-xs text-gray-400 space-y-1">
                  <li>• Save path should contain your save folders (e.g., Survival, Builder)</li>
                  <li>• Backup path will be created automatically if it doesn't exist</li>
                  <li>• Changes take effect immediately for new operations</li>
                </ul>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 p-6 border-t border-gray-800">
          <button
            type="button"
            onClick={handleCancel}
            disabled={isSaving}
            className="px-6 py-2 bg-gray-800 hover:bg-gray-700 text-foreground rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={isSaving || isLoading}
            className="px-6 py-2 bg-primary hover:bg-red-700 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSaving ? "Saving..." : "Save Settings"}
          </button>
        </div>
      </div>
    </div>
  );
};
