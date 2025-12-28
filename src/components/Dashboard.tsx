import { useState } from "react";
import { BackupList } from "./BackupList";
import { SaveSelector } from "./SaveSelector";

interface DashboardProps {
  onRestoreInitiate?: (saveName: string, backupName: string) => void;
}

/**
 * Dashboard component
 * Main content area showing save selector and backup list
 */
export const Dashboard: React.FC<DashboardProps> = ({ onRestoreInitiate }) => {
  const [selectedSave, setSelectedSave] = useState<string | null>(null);
  const [key, setKey] = useState(0); // Used to force re-render

  const handleSaveChange = (saveName: string | null) => {
    setSelectedSave(saveName);
  };

  const handleRestore = (saveName: string, backupName: string) => {
    if (onRestoreInitiate) {
      onRestoreInitiate(saveName, backupName);
    }
  };

  // Force refresh when the component is remounted
  const handleRefresh = () => {
    setKey((prev) => prev + 1);
  };

  return (
    <div className="h-full flex flex-col p-6 space-y-6" key={key}>
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

      {/* Save Selector */}
      <SaveSelector selectedSave={selectedSave} onSaveChange={handleSaveChange} />

      {/* Backup List */}
      <div className="flex-1 min-h-0">
        <BackupList saveName={selectedSave} onRestore={handleRestore} />
      </div>

      {/* Footer info */}
      <div className="text-center text-xs text-gray-600 pt-4 border-t border-gray-800">
        Select a save and view its backup history above
      </div>
    </div>
  );
};
