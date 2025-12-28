import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

interface BackupListProps {
  saveName: string | null;
  onRestore?: (saveName: string, backupName: string) => void;
}

interface BackupInfo {
  name: string;
  size_bytes: number;
  size_formatted: string;
  created_at: string;
  backup_path: string;
}

interface BackupItem {
  name: string;
  sizeFormatted: string;
  createdAt: string;
  timeAgo: string;
}

/**
 * Formats a timestamp as a human-readable "time ago" string
 */
function formatTimeAgo(timestamp: string): string {
  const now = new Date();
  const past = new Date(timestamp);
  const diffMs = now.getTime() - past.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);
  const diffHours = Math.floor(diffMins / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffSecs < 60) {
    return "just now";
  } else if (diffMins < 60) {
    return `${diffMins} minute${diffMins !== 1 ? "s" : ""} ago`;
  } else if (diffHours < 24) {
    return `${diffHours} hour${diffHours !== 1 ? "s" : ""} ago`;
  } else if (diffDays < 7) {
    return `${diffDays} day${diffDays !== 1 ? "s" : ""} ago`;
  } else {
    return past.toLocaleDateString();
  }
}

/**
 * Formats a timestamp for display (YYYY-MM-DD HH:mm)
 */
function formatDateTime(timestamp: string): string {
  const date = new Date(timestamp);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  return `${year}-${month}-${day} ${hours}:${minutes}`;
}

/**
 * BackupList component
 * Displays a list of backups for the selected save
 */
export const BackupList: React.FC<BackupListProps> = ({ saveName, onRestore }) => {
  const [backups, setBackups] = useState<BackupItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadBackups = useCallback(async () => {
    if (!saveName) return;

    try {
      setLoading(true);
      setError(null);
      const backupInfos: BackupInfo[] = await invoke("list_backups_command", {
        saveName,
      });

      const items: BackupItem[] = backupInfos.map((info) => ({
        name: info.name,
        sizeFormatted: info.size_formatted,
        createdAt: formatDateTime(info.created_at),
        timeAgo: formatTimeAgo(info.created_at),
      }));

      setBackups(items);
    } catch (err) {
      console.error("Failed to load backups:", err);
      setError("Failed to load backups");
      setBackups([]);
    } finally {
      setLoading(false);
    }
  }, [saveName]);

  useEffect(() => {
    if (saveName) {
      loadBackups();
    } else {
      setBackups([]);
    }
  }, [saveName, loadBackups]);

  const handleRestore = (backup: BackupItem) => {
    if (onRestore && saveName) {
      onRestore(saveName, backup.name);
    }
  };

  if (!saveName) {
    return (
      <div className="bg-gray-900 border border-gray-800 rounded-lg p-8">
        <div className="text-center text-gray-500">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="w-16 h-16 mx-auto mb-4 opacity-50"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
            />
          </svg>
          <p className="text-lg font-medium">No save selected</p>
          <p className="text-sm mt-2">Select a save to view backup history</p>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="bg-gray-900 border border-gray-800 rounded-lg p-8">
        <div className="flex items-center justify-center text-gray-400">
          <svg
            className="animate-spin h-6 w-6 mr-3"
            viewBox="0 0 24 24"
            fill="none"
            aria-hidden="true"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
          </svg>
          <span>Loading backups...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-gray-900 border border-gray-800 rounded-lg p-8">
        <div className="text-center text-warning">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className="w-12 h-12 mx-auto mb-4"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          <p className="font-medium mb-2">Failed to load backups</p>
          <p className="text-sm text-gray-400 mb-4">{error}</p>
          <button
            type="button"
            onClick={loadBackups}
            className="px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded text-gray-300 transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (backups.length === 0) {
    return (
      <div className="bg-gray-900 border border-gray-800 rounded-lg p-8">
        <div className="text-center text-gray-500">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            className="w-16 h-16 mx-auto mb-4 opacity-50"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p className="text-lg font-medium">No backups yet</p>
          <p className="text-sm mt-2">Create your first backup for "{saveName}" to get started</p>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-lg overflow-hidden">
      {/* Header */}
      <div className="px-6 py-4 border-b border-gray-800">
        <h2 className="text-lg font-semibold text-foreground">Backup History ({backups.length})</h2>
      </div>

      {/* List */}
      <div className="divide-y divide-gray-800">
        {backups.map((backup) => (
          <div key={backup.name} className="px-6 py-4 hover:bg-gray-800/50 transition-colors group">
            <div className="flex items-center justify-between">
              {/* Left: Time info */}
              <div className="flex-1">
                <div className="flex items-center space-x-3">
                  <div className="flex items-center space-x-2">
                    <svg
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      className="w-5 h-5 text-gray-500"
                      aria-hidden="true"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                      />
                    </svg>
                    <span className="font-medium text-foreground">{backup.createdAt}</span>
                  </div>
                  <span className="text-sm text-gray-500">({backup.timeAgo})</span>
                </div>
              </div>

              {/* Center: Size */}
              <div className="flex-shrink-0 mx-6">
                <span className="text-sm font-medium text-gray-400">{backup.sizeFormatted}</span>
              </div>

              {/* Right: Actions */}
              <div className="flex-shrink-0 flex items-center space-x-2">
                <button
                  type="button"
                  onClick={() => handleRestore(backup)}
                  className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded transition-colors"
                >
                  Restore
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
