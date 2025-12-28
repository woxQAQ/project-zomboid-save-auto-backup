import { useEffect } from "react";

interface RestoreModalProps {
  isOpen: boolean;
  saveName: string | null;
  backupName: string | null;
  backupTime?: string;
  onConfirm: () => void;
  onCancel: () => void;
  isRestoring?: boolean;
}

/**
 * RestoreModal component
 * Confirmation dialog for restore operations with safety warnings
 */
export const RestoreModal: React.FC<RestoreModalProps> = ({
  isOpen,
  saveName,
  backupName,
  backupTime,
  onConfirm,
  onCancel,
  isRestoring = false,
}) => {
  // Handle Escape key to close modal
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !isRestoring) {
        onCancel();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isOpen, onCancel, isRestoring]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/70"
      role="dialog"
      aria-modal="true"
      aria-labelledby="restore-modal-title"
      aria-describedby="restore-modal-description"
    >
      <div className="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl max-w-md w-full">
        {/* Header with warning icon */}
        <div className="p-6 border-b border-gray-800">
          <div className="flex items-start space-x-4">
            <div className="flex-shrink-0">
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                className="w-8 h-8 text-warning"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                />
              </svg>
            </div>
            <div className="flex-1">
              <h3 id="restore-modal-title" className="text-lg font-semibold text-foreground">
                Confirm Restore
              </h3>
            </div>
          </div>
        </div>

        {/* Body */}
        <div className="p-6" id="restore-modal-description">
          <p className="text-gray-300 mb-4">
            This operation will <span className="text-red-400 font-semibold">overwrite</span> the
            current{" "}
            <span className="font-mono text-sm bg-gray-800 px-2 py-1 rounded">{saveName}</span>{" "}
            save.
          </p>

          <div className="bg-gray-800 border border-gray-700 rounded p-4 mb-4">
            <p className="text-sm text-gray-400 mb-1">Restoring to backup:</p>
            <p className="font-mono text-sm text-foreground break-all">{backupName}</p>
            {backupTime && <p className="text-xs text-gray-500 mt-2">Created: {backupTime}</p>}
          </div>

          {/* Safety warning */}
          <div className="flex items-start space-x-3 bg-yellow-900/20 border border-yellow-900/50 rounded p-3">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              className="w-5 h-5 text-yellow-600 flex-shrink-0 mt-0.5"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
            <div>
              <p className="text-sm text-yellow-200 font-medium">Automatic safety backup</p>
              <p className="text-xs text-gray-400 mt-1">
                Before restoring, we'll automatically create an "undo snapshot" of your current
                save. If something goes wrong, you can roll back.
              </p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-gray-800 flex justify-end space-x-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={isRestoring}
            className="px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-300 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={isRestoring}
            className="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center space-x-2"
          >
            {isRestoring ? (
              <>
                <svg
                  className="animate-spin h-4 w-4"
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
                  className="w-4 h-4"
                  aria-hidden="true"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                  />
                </svg>
                <span>Confirm Restore</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
