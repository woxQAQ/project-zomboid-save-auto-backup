import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { AutoBackupControl, DeleteModal, RestoreModal, Toast, type ToastType } from "./";
import { BackupList } from "./BackupList";
import { SaveSelector } from "./SaveSelector";

interface BackupResult {
  backup_path: string;
  backup_name: string;
  retained_count: number;
  deleted_count: number;
}

/**
 * Dashboard component
 * Main content area showing save selector, backup actions, and backup list
 */
export const Dashboard: React.FC = () => {
  const [selectedSave, setSelectedSave] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0); // Used to force refresh components

  // Backup Now state
  const [isBackingUp, setIsBackingUp] = useState(false);

  // Restore modal state
  const [showRestoreModal, setShowRestoreModal] = useState(false);
  const [restoreData, setRestoreData] = useState<{
    saveName: string;
    backupName: string;
    backupTime: string;
  } | null>(null);
  const [isRestoring, setIsRestoring] = useState(false);

  // Delete modal state
  const [showDeleteModal, setShowDeleteModal] = useState(false);
  const [deleteData, setDeleteData] = useState<{
    saveName: string;
    backupName: string;
    backupTime: string;
  } | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  // Toast state
  const [toast, setToast] = useState<{
    message: string;
    type: ToastType;
  } | null>(null);

  const hideToast = useCallback(() => setToast(null), []);

  // Format error message for display (truncate if too long)
  const formatErrorMessage = useCallback((err: unknown): string => {
    const errStr = String(err);
    return errStr.length > 100 ? `${errStr.substring(0, 100)}...` : errStr;
  }, []);

  const showToast = useCallback((message: string, type: ToastType = "info") => {
    setToast({ message, type });
  }, []);

  const handleSaveChange = (saveName: string | null) => {
    setSelectedSave(saveName);
  };

  // Refresh auto backup saves when selected save changes
  // biome-ignore lint/correctness/useExhaustiveDependencies: selectedSave is intentionally tracked to refresh saves on change
  useEffect(() => {
    const refreshAutoBackupSaves = async () => {
      try {
        await invoke("refresh_auto_backup_saves");
      } catch (err) {
        console.error("Failed to refresh auto backup saves:", err);
      }
    };
    refreshAutoBackupSaves();
  }, [selectedSave]);

  // Backup now handler
  const handleBackupNow = async () => {
    if (!selectedSave) {
      showToast("Please select a save first", "warning");
      return;
    }

    try {
      setIsBackingUp(true);
      const result: BackupResult = await invoke("create_backup_command", {
        saveName: selectedSave,
      });

      // Trigger refresh of backup list
      refreshBackupList();

      showToast(
        `Backup created successfully! (${result.retained_count} backup${result.retained_count !== 1 ? "s" : ""} retained)`,
        "success",
      );
    } catch (err) {
      console.error("Backup failed:", err);
      showToast(`Backup failed: ${formatErrorMessage(err)}`, "error");
    } finally {
      setIsBackingUp(false);
    }
  };

  // Restore handler - shows confirmation modal
  const handleRestore = (saveName: string, backupName: string, backupTime: string) => {
    setRestoreData({ saveName, backupName, backupTime });
    setShowRestoreModal(true);
  };

  // Confirm restore handler
  const handleConfirmRestore = async () => {
    if (!restoreData) return;

    try {
      setIsRestoring(true);
      await invoke("restore_backup_command", {
        saveName: restoreData.saveName,
        backupName: restoreData.backupName,
      });

      // Trigger refresh of backup list
      refreshBackupList();

      showToast(
        `Restore completed! Your save has been restored to ${restoreData.backupTime}`,
        "success",
      );
      setShowRestoreModal(false);
      setRestoreData(null);
    } catch (err) {
      console.error("Restore failed:", err);
      showToast(`Restore failed: ${formatErrorMessage(err)}`, "error");
    } finally {
      setIsRestoring(false);
    }
  };

  // Cancel restore handler
  const handleCancelRestore = () => {
    setShowRestoreModal(false);
    setRestoreData(null);
  };

  // Delete handler - shows confirmation modal
  const handleDelete = (saveName: string, backupName: string, backupTime: string) => {
    setDeleteData({ saveName, backupName, backupTime });
    setShowDeleteModal(true);
  };

  // Confirm delete handler
  const handleConfirmDelete = async () => {
    if (!deleteData) return;

    try {
      setIsDeleting(true);
      await invoke("delete_backup_command", {
        saveName: deleteData.saveName,
        backupName: deleteData.backupName,
      });

      // Trigger refresh of backup list
      refreshBackupList();

      showToast("Backup deleted successfully", "success");
      setShowDeleteModal(false);
      setDeleteData(null);
    } catch (err) {
      console.error("Delete failed:", err);
      showToast(`Delete failed: ${formatErrorMessage(err)}`, "error");
    } finally {
      setIsDeleting(false);
    }
  };

  // Cancel delete handler
  const handleCancelDelete = () => {
    setShowDeleteModal(false);
    setDeleteData(null);
  };

  // Force refresh of dashboard components
  const handleRefresh = () => {
    setRefreshKey((prev) => prev + 1);
  };

  // Function to trigger backup list refresh after mutations
  const refreshBackupList = () => {
    setRefreshKey((prev) => prev + 1);
  };

  return (
    <div className="h-full flex flex-col p-6 space-y-6" key={refreshKey}>
      {/* Page Title */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-foreground">Dashboard</h1>
          <p className="text-sm text-gray-400 mt-1">Manage your Project Zomboid save backups</p>
        </div>
        <button
          type="button"
          onClick={handleRefresh}
          className="px-4 py-2 bg-gray-800 hover:bg-gray-700 rounded text-gray-300 transition-colors flex items-center space-x-2"
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className="w-4 h-4"
            aria-hidden="true"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          </svg>
          <span>Refresh</span>
        </button>
      </div>

      {/* Save Selector + Backup Now Button */}
      <div className="flex items-center gap-4">
        <div className="flex-1">
          <SaveSelector selectedSave={selectedSave} onSaveChange={handleSaveChange} />
        </div>
        <button
          type="button"
          onClick={handleBackupNow}
          disabled={!selectedSave || isBackingUp}
          className="px-6 py-3 bg-primary hover:bg-red-700 disabled:bg-gray-800 disabled:text-gray-600 text-white font-semibold rounded-lg transition-colors flex items-center space-x-2 whitespace-nowrap"
        >
          {isBackingUp ? (
            <>
              <svg
                className="animate-spin h-5 w-5"
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
              <span>Backing up...</span>
            </>
          ) : (
            <>
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                className="w-5 h-5"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
                />
              </svg>
              <span>Backup Now</span>
            </>
          )}
        </button>
      </div>

      {/* Auto Backup Control */}
      <AutoBackupControl selectedSave={selectedSave} showToast={showToast} />

      {/* Backup List */}
      <div className="flex-1 min-h-0">
        <BackupList
          key={refreshKey}
          saveName={selectedSave}
          onRestore={handleRestore}
          onDelete={handleDelete}
          deletingBackup={isDeleting ? (deleteData?.backupName ?? null) : null}
        />
      </div>

      {/* Footer info */}
      <div className="text-center text-xs text-gray-600 pt-4 border-t border-gray-800">
        Select a save and view its backup history above
      </div>

      {/* Restore Confirmation Modal */}
      {restoreData && (
        <RestoreModal
          isOpen={showRestoreModal}
          saveName={restoreData.saveName}
          backupName={restoreData.backupName}
          backupTime={restoreData.backupTime}
          onConfirm={handleConfirmRestore}
          onCancel={handleCancelRestore}
          isRestoring={isRestoring}
        />
      )}

      {/* Delete Confirmation Modal */}
      {deleteData && (
        <DeleteModal
          isOpen={showDeleteModal}
          backupName={deleteData.backupName}
          backupTime={deleteData.backupTime}
          onConfirm={handleConfirmDelete}
          onCancel={handleCancelDelete}
          isDeleting={isDeleting}
        />
      )}

      {/* Toast Notification */}
      {toast && <Toast message={toast.message} type={toast.type} onClose={hideToast} />}
    </div>
  );
};
