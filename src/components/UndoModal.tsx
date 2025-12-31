import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

interface UndoSnapshotInfo {
  name: string;
  path: string;
  size_bytes: number;
  size_formatted: string;
  created_at: string;
  save_name: string;
}

interface UndoModalProps {
  isOpen: boolean;
  saveName: string;
  onClose: () => void;
  onUndoRestored?: () => void;
}

/**
 * UndoModal component
 * Displays undo snapshots for a save with restore and delete functionality
 */
export const UndoModal: React.FC<UndoModalProps> = ({
  isOpen,
  saveName,
  onClose,
  onUndoRestored,
}) => {
  const [snapshots, setSnapshots] = useState<UndoSnapshotInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [restoringSnapshot, setRestoringSnapshot] = useState<string | null>(null);
  const [deletingSnapshot, setDeletingSnapshot] = useState<string | null>(null);

  // Load snapshots when modal opens
  useEffect(() => {
    if (!isOpen) return;

    const loadSnapshots = async () => {
      setIsLoading(true);
      setError(null);
      try {
        const result: UndoSnapshotInfo[] = await invoke("list_undo_snapshots_command", {
          saveName,
        });
        setSnapshots(result);
      } catch (err) {
        console.error("Failed to load undo snapshots:", err);
        setError(String(err));
      } finally {
        setIsLoading(false);
      }
    };

    loadSnapshots();
  }, [isOpen, saveName]);

  // Handle Escape key to close modal
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !restoringSnapshot && !deletingSnapshot) {
        onClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onClose, restoringSnapshot, deletingSnapshot]);

  const handleRestore = async (snapshotName: string) => {
    setRestoringSnapshot(snapshotName);
    setError(null);
    try {
      await invoke("restore_from_undo_snapshot_command", {
        saveName,
        snapshotName,
      });
      // Refresh the list
      const result: UndoSnapshotInfo[] = await invoke("list_undo_snapshots_command", {
        saveName,
      });
      setSnapshots(result);
      onUndoRestored?.();
    } catch (err) {
      console.error("Failed to restore from undo snapshot:", err);
      setError(String(err));
    } finally {
      setRestoringSnapshot(null);
    }
  };

  const handleDelete = async (snapshotName: string) => {
    if (!confirm(`Are you sure you want to delete this undo snapshot?\n\n${snapshotName}`)) {
      return;
    }
    setDeletingSnapshot(snapshotName);
    setError(null);
    try {
      await invoke("delete_undo_snapshot_command", {
        saveName,
        snapshotName,
      });
      // Refresh the list
      const result: UndoSnapshotInfo[] = await invoke("list_undo_snapshots_command", {
        saveName,
      });
      setSnapshots(result);
    } catch (err) {
      console.error("Failed to delete undo snapshot:", err);
      setError(String(err));
    } finally {
      setDeletingSnapshot(null);
    }
  };

  const formatDate = (isoString: string) => {
    const date = new Date(isoString);
    return date.toLocaleString();
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70"
      role="dialog"
      aria-modal="true"
      aria-labelledby="undo-modal-title"
    >
      <div className="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl max-w-2xl w-full max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="p-6 border-b border-gray-800 flex items-start justify-between">
          <div className="flex items-start space-x-4">
            <div className="flex-shrink-0">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                className="w-8 h-8 text-blue-400"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
            </div>
            <div className="flex-1">
              <h3 id="undo-modal-title" className="text-lg font-semibold text-foreground">
                Undo History
              </h3>
              <p className="text-sm text-gray-400 mt-1">
                Save: <span className="font-mono">{saveName}</span>
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={restoringSnapshot !== null || deletingSnapshot !== null}
            className="text-gray-400 hover:text-gray-200 transition-colors disabled:opacity-50"
          >
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              className="w-6 h-6"
              aria-hidden="true"
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="p-6 flex-1 overflow-y-auto">
          {error && (
            <div className="mb-4 bg-red-900/30 border border-red-800 rounded p-3 text-sm text-red-300">
              {error}
            </div>
          )}

          {isLoading ? (
            <div className="flex items-center justify-center py-8">
              <svg
                className="animate-spin h-8 w-8 text-primary"
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
            </div>
          ) : snapshots.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                className="w-12 h-12 mx-auto mb-3 opacity-50"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <p>No undo snapshots found</p>
              <p className="text-xs mt-2">
                Undo snapshots are automatically created before each restore operation
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {snapshots.map((snapshot) => (
                <div
                  key={snapshot.name}
                  className="bg-gray-800 border border-gray-700 rounded-lg p-4"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center space-x-2 mb-2">
                        <svg
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2"
                          className="w-4 h-4 text-blue-400 flex-shrink-0"
                          aria-hidden="true"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"
                          />
                        </svg>
                        <span className="text-sm font-mono text-foreground truncate">
                          {snapshot.name}
                        </span>
                      </div>
                      <div className="grid grid-cols-2 gap-2 text-xs text-gray-400">
                        <div>
                          <span className="text-gray-500">Created:</span>{" "}
                          {formatDate(snapshot.created_at)}
                        </div>
                        <div>
                          <span className="text-gray-500">Size:</span> {snapshot.size_formatted}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center space-x-2 ml-4">
                      <button
                        type="button"
                        onClick={() => handleRestore(snapshot.name)}
                        disabled={restoringSnapshot === snapshot.name || deletingSnapshot !== null}
                        className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:text-gray-500 text-white text-sm rounded transition-colors flex items-center space-x-1"
                        title="Restore from this snapshot"
                      >
                        {restoringSnapshot === snapshot.name ? (
                          <>
                            <svg
                              className="animate-spin h-3 w-3"
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
                            <span>Restoring...</span>
                          </>
                        ) : (
                          <>
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
                                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                              />
                            </svg>
                            <span>Restore</span>
                          </>
                        )}
                      </button>
                      <button
                        type="button"
                        onClick={() => handleDelete(snapshot.name)}
                        disabled={deletingSnapshot === snapshot.name || restoringSnapshot !== null}
                        className="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 disabled:bg-gray-800 disabled:text-gray-600 text-gray-300 text-sm rounded transition-colors flex items-center space-x-1"
                        title="Delete this snapshot"
                      >
                        {deletingSnapshot === snapshot.name ? (
                          <>
                            <svg
                              className="animate-spin h-3 w-3"
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
                            <span>Deleting...</span>
                          </>
                        ) : (
                          <>
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
                                d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                              />
                            </svg>
                            <span>Delete</span>
                          </>
                        )}
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-gray-800 flex justify-end">
          <button
            type="button"
            onClick={onClose}
            disabled={restoringSnapshot !== null || deletingSnapshot !== null}
            className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
