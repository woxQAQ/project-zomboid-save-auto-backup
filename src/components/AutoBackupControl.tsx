import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

interface SaveAutoBackupState {
  save_name: string;
  enabled: boolean;
  last_backup_time: string | null;
  next_backup_time: string | null;
}

interface AutoBackupStatus {
  is_running: boolean;
  interval_seconds: number;
  saves: Record<string, SaveAutoBackupState>;
  started_at: string | null;
}

export type ToastType = "success" | "error" | "warning" | "info";

interface AutoBackupControlProps {
  selectedSave: string | null;
  showToast?: (message: string, type: ToastType) => void;
}

/**
 * AutoBackupControl component
 * Displays auto backup status and controls for the selected save
 */
export const AutoBackupControl: React.FC<AutoBackupControlProps> = ({
  selectedSave,
  showToast: externalShowToast,
}) => {
  const [status, setStatus] = useState<AutoBackupStatus | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isTogglingService, setIsTogglingService] = useState(false);
  const [isTogglingSave, setIsTogglingSave] = useState(false);
  const [intervalInput, setIntervalInput] = useState("300");
  const [showIntervalEdit, setShowIntervalEdit] = useState(false);
  const [isSavingInterval, setIsSavingInterval] = useState(false);
  const intervalInputRef = useRef<HTMLInputElement>(null);

  // Format error message for display (truncate if too long)
  const formatErrorMessage = useCallback((err: unknown): string => {
    const errStr = String(err);
    return errStr.length > 100 ? `${errStr.substring(0, 100)}...` : errStr;
  }, []);

  // Helper to show toast (if callback provided) or log to console
  const notify = useCallback(
    (message: string, type: ToastType = "info") => {
      if (externalShowToast) {
        externalShowToast(message, type);
      }
    },
    [externalShowToast],
  );

  // Load auto backup status
  const loadStatus = useCallback(async () => {
    try {
      const result: AutoBackupStatus = await invoke("get_auto_backup_status");
      setStatus(result);
      setIntervalInput(String(result.interval_seconds));
    } catch (err) {
      console.error("Failed to load auto backup status:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadStatus();
  }, [loadStatus]);

  // Refresh status periodically when running
  useEffect(() => {
    if (!status?.is_running) return;

    const interval = setInterval(() => {
      loadStatus();
    }, 5000); // Refresh every 5 seconds

    return () => clearInterval(interval);
  }, [status?.is_running, loadStatus]);

  // Focus interval input when edit mode is enabled
  useEffect(() => {
    if (showIntervalEdit) {
      intervalInputRef.current?.focus();
    }
  }, [showIntervalEdit]);

  // Format interval to human readable
  const formatInterval = useCallback((seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const hours = Math.floor(mins / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) return `${days}d`;
    if (hours > 0) return `${hours}h`;
    return `${mins}m`;
  }, []);

  // Start/Stop auto backup service
  const handleToggleService = async () => {
    try {
      setIsTogglingService(true);
      if (status?.is_running) {
        await invoke("stop_auto_backup");
        notify("Auto backup service stopped", "success");
      } else {
        await invoke("start_auto_backup");
        notify("Auto backup service started", "success");
      }
      await loadStatus();
    } catch (err) {
      console.error("Failed to toggle auto backup service:", err);
      notify(`Failed to toggle service: ${formatErrorMessage(err)}`, "error");
    } finally {
      setIsTogglingService(false);
    }
  };

  // Enable/Disable auto backup for selected save
  const handleToggleSave = async () => {
    if (!selectedSave) return;

    try {
      setIsTogglingSave(true);
      const currentState = status?.saves[selectedSave]?.enabled ?? false;
      if (currentState) {
        await invoke("disable_auto_backup", { saveName: selectedSave });
        notify(`Auto backup disabled for '${selectedSave}'`, "success");
      } else {
        await invoke("enable_auto_backup", { saveName: selectedSave });
        notify(`Auto backup enabled for '${selectedSave}'`, "success");
      }
      await loadStatus();
    } catch (err) {
      console.error("Failed to toggle save auto backup:", err);
      notify(`Failed to toggle auto backup: ${formatErrorMessage(err)}`, "error");
    } finally {
      setIsTogglingSave(false);
    }
  };

  // Save interval change
  const handleSaveInterval = async () => {
    try {
      setIsSavingInterval(true);
      const seconds = parseInt(intervalInput, 10);
      if (Number.isNaN(seconds) || seconds < 60 || seconds > 86400) {
        notify("Interval must be between 60 and 86400 seconds (1 minute to 24 hours)", "warning");
        return;
      }
      await invoke("set_auto_backup_interval", { seconds });
      await loadStatus();
      setShowIntervalEdit(false);
      notify(`Backup interval updated to ${formatInterval(seconds)}`, "success");
    } catch (err) {
      console.error("Failed to set interval:", err);
      notify(`Failed to update interval: ${formatErrorMessage(err)}`, "error");
    } finally {
      setIsSavingInterval(false);
    }
  };

  // Format timestamp to relative time
  const formatTimeAgo = useCallback((timestamp: string | null): string => {
    if (!timestamp) return "Never";

    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${diffDays}d ago`;
  }, []);

  // Calculate time until next backup
  const getTimeUntilNext = useCallback((nextTime: string | null): string => {
    if (!nextTime) return "Not scheduled";

    const date = new Date(nextTime);
    const now = new Date();
    const diffMs = date.getTime() - now.getTime();

    if (diffMs <= 0) return "Due now";

    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);

    if (diffDays > 0) return `${diffDays}d ${diffHours % 24}h`;
    if (diffHours > 0) return `${diffHours}h ${diffMins % 60}m`;
    return `${diffMins}m`;
  }, []);

  if (isLoading) {
    return (
      <div className="bg-gray-900/50 border border-gray-800 rounded-lg p-4">
        <div className="animate-pulse flex space-x-4">
          <div className="flex-1 space-y-3">
            <div className="h-4 bg-gray-800 rounded w-1/3"></div>
            <div className="h-3 bg-gray-800 rounded w-1/2"></div>
          </div>
        </div>
      </div>
    );
  }

  if (!status) {
    return (
      <div className="bg-gray-900/50 border border-gray-800 rounded-lg p-4">
        <p className="text-sm text-gray-500">Unable to load auto backup status</p>
      </div>
    );
  }

  const saveState = selectedSave ? status.saves[selectedSave] : null;
  const isSaveEnabled = saveState?.enabled ?? false;

  return (
    <div className="bg-gray-900/50 border border-gray-800 rounded-lg p-4">
      {/* Header: Service toggle and interval */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center space-x-3">
          <h3 className="text-sm font-semibold text-foreground">Auto Backup</h3>
          <span
            className={`px-2 py-0.5 text-xs rounded-full ${
              status.is_running ? "bg-green-900/50 text-green-400" : "bg-gray-800 text-gray-500"
            }`}
          >
            {status.is_running ? "Running" : "Stopped"}
          </span>
        </div>
        <div className="flex items-center space-x-3">
          {/* Interval display/edit */}
          {showIntervalEdit ? (
            <div className="flex items-center space-x-2">
              <input
                ref={intervalInputRef}
                type="number"
                min="60"
                max="86400"
                value={intervalInput}
                onChange={(e) => setIntervalInput(e.target.value)}
                className="w-20 px-2 py-1 bg-gray-800 border border-gray-700 rounded text-sm text-foreground focus:outline-none focus:border-primary"
                disabled={isSavingInterval}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleSaveInterval();
                  if (e.key === "Escape") setShowIntervalEdit(false);
                }}
              />
              <span className="text-xs text-gray-500">seconds</span>
              <button
                type="button"
                onClick={handleSaveInterval}
                disabled={isSavingInterval}
                className="p-1 hover:bg-gray-800 rounded text-gray-400 hover:text-green-400 transition-colors"
                aria-label="Save interval"
              >
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  className="w-4 h-4"
                  aria-hidden="true"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
                </svg>
              </button>
              <button
                type="button"
                onClick={() => setShowIntervalEdit(false)}
                className="p-1 hover:bg-gray-800 rounded text-gray-400 hover:text-red-400 transition-colors"
                aria-label="Cancel"
              >
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  className="w-4 h-4"
                  aria-hidden="true"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => setShowIntervalEdit(true)}
              className="px-2 py-1 text-xs bg-gray-800 hover:bg-gray-700 rounded text-gray-400 transition-colors flex items-center space-x-1"
              aria-label="Edit backup interval"
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                className="w-3 h-3"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <span>Every {formatInterval(status.interval_seconds)}</span>
            </button>
          )}
        </div>
      </div>

      {/* Service control */}
      <div className="flex items-center justify-between py-3 border-t border-gray-800">
        <div className="flex-1">
          <p className="text-sm text-gray-300">
            {status.is_running
              ? "Auto backup service is running in the background"
              : "Auto backup service is stopped"}
          </p>
          {status.is_running && status.started_at && (
            <p className="text-xs text-gray-500 mt-1">Started {formatTimeAgo(status.started_at)}</p>
          )}
        </div>
        <button
          type="button"
          onClick={handleToggleService}
          disabled={isTogglingService}
          className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
            status.is_running
              ? "bg-red-900/30 hover:bg-red-900/50 text-red-400 border border-red-900/50"
              : "bg-green-900/30 hover:bg-green-900/50 text-green-400 border border-green-900/50"
          } ${isTogglingService ? "opacity-50 cursor-not-allowed" : ""}`}
        >
          {isTogglingService ? "..." : status.is_running ? "Stop Service" : "Start Service"}
        </button>
      </div>

      {/* Save-specific control */}
      {selectedSave && (
        <div className="flex items-center justify-between py-3 border-t border-gray-800 mt-3">
          <div className="flex-1">
            <p className="text-sm text-gray-300">
              Auto backup for <span className="font-semibold text-foreground">{selectedSave}</span>
            </p>
            {isSaveEnabled && saveState && (
              <div className="flex items-center space-x-4 mt-1">
                <p className="text-xs text-gray-500">
                  Last: {formatTimeAgo(saveState.last_backup_time)}
                </p>
                {saveState.next_backup_time && (
                  <p className="text-xs text-gray-500">
                    Next: {getTimeUntilNext(saveState.next_backup_time)}
                  </p>
                )}
              </div>
            )}
          </div>
          <button
            type="button"
            onClick={handleToggleSave}
            disabled={isTogglingSave || !status.is_running}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              isSaveEnabled
                ? "bg-green-900/30 hover:bg-green-900/50 text-green-400 border border-green-900/50"
                : "bg-gray-800 hover:bg-gray-700 text-gray-400 border border-gray-700"
            } ${isTogglingSave || !status.is_running ? "opacity-50 cursor-not-allowed" : ""}`}
          >
            {isTogglingSave ? "..." : isSaveEnabled ? "Enabled" : "Enable"}
          </button>
        </div>
      )}

      {/* Info message when no save selected */}
      {!selectedSave && (
        <div className="py-3 border-t border-gray-800 mt-3">
          <p className="text-xs text-gray-500">
            Select a save to configure its auto backup settings
          </p>
        </div>
      )}
    </div>
  );
};
