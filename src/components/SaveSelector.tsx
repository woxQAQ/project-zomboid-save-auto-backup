import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

interface SaveSelectorProps {
  selectedSave: string | null;
  onSaveChange: (saveName: string | null) => void;
}

interface SaveEntry {
  game_mode: string;
  save_name: string;
  relative_path: string;
}

/**
 * SaveSelector component
 * Displays a cascaded dropdown selector for choosing saves grouped by game mode
 * Supports the two-level directory structure: Saves/<GameMode>/<SaveName>
 */
export const SaveSelector: React.FC<SaveSelectorProps> = ({ selectedSave, onSaveChange }) => {
  const [savesByGameMode, setSavesByGameMode] = useState<Record<string, SaveEntry[]>>({});
  const [allSaves, setAllSaves] = useState<SaveEntry[]>([]);
  const [selectedGameModeKey, setSelectedGameModeKey] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Helper function to get display name for game mode
  // Note: Backend already returns "(Other)" for flat saves, so this is mainly for consistency
  const getGameModeDisplay = useCallback((gameMode: string): string => {
    return gameMode || "(Other)";
  }, []);

  const loadSaves = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const grouped: Record<string, SaveEntry[]> = await invoke("list_save_entries_by_game_mode");

      // Convert to array and sort
      const saveEntries: SaveEntry[] = Object.values(grouped).flat();

      setSavesByGameMode(grouped);
      setAllSaves(saveEntries);

      // Auto-select the first save if none is selected
      if (!selectedSave && saveEntries.length > 0) {
        const firstEntry = saveEntries[0];
        // Store the raw game mode key (from backend) for lookups
        setSelectedGameModeKey(firstEntry.game_mode);
        onSaveChange(firstEntry.relative_path);
      } else if (selectedSave) {
        // Find the game mode for the currently selected save
        const entry = saveEntries.find((e) => e.relative_path === selectedSave);
        if (entry) {
          setSelectedGameModeKey(entry.game_mode);
        } else {
          // If the currently selected save no longer exists, clear selection
          setSelectedGameModeKey("");
          onSaveChange(null);
        }
      }
    } catch (err) {
      console.error("Failed to load saves:", err);
      setError("Failed to load saves. Please check your save path in settings.");
    } finally {
      setLoading(false);
    }
  }, [selectedSave, onSaveChange]);

  useEffect(() => {
    loadSaves();
  }, [loadSaves]);

  // Get list of available game modes
  const gameModes = Object.keys(savesByGameMode).sort();

  // Get saves for selected game mode (use raw key for lookup)
  const savesForSelectedMode = selectedGameModeKey
    ? savesByGameMode[selectedGameModeKey] || []
    : [];

  const handleGameModeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const gameMode = e.target.value;
    setSelectedGameModeKey(gameMode);

    // Auto-select the first save in this game mode
    const saves = savesByGameMode[gameMode];
    if (saves && saves.length > 0) {
      onSaveChange(saves[0].relative_path);
    } else {
      onSaveChange(null);
    }
  };

  const handleSaveChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    onSaveChange(value || null);
  };

  if (loading) {
    return (
      <div className="bg-gray-900 border border-gray-800 rounded-lg p-4 flex items-center justify-center">
        <div className="flex items-center space-x-2 text-gray-400">
          <svg className="animate-spin h-5 w-5" viewBox="0 0 24 24" fill="none" aria-hidden="true">
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
          <span>Loading saves...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-gray-900 border border-gray-800 rounded-lg p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2 text-warning">
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
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            <span className="text-sm">{error}</span>
          </div>
          <button
            type="button"
            onClick={loadSaves}
            className="text-xs px-3 py-1 bg-gray-800 hover:bg-gray-700 rounded text-gray-300 transition-colors"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  if (allSaves.length === 0) {
    return (
      <div className="bg-gray-900 border border-gray-800 rounded-lg p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2 text-gray-500">
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
                d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"
              />
            </svg>
            <span className="text-sm">No saves found</span>
          </div>
          <button
            type="button"
            onClick={loadSaves}
            className="text-xs px-3 py-1 bg-gray-800 hover:bg-gray-700 rounded text-gray-300 transition-colors"
          >
            Refresh
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-lg p-4">
      <div className="flex items-center gap-4">
        {/* Game Mode Selector */}
        <div className="flex-1">
          <label
            htmlFor="game-mode-select"
            className="block text-sm font-medium text-gray-400 mb-2"
          >
            Game Mode
          </label>
          <select
            id="game-mode-select"
            value={selectedGameModeKey}
            onChange={handleGameModeChange}
            disabled={gameModes.length === 0}
            className="w-full bg-gray-800 border border-gray-700 rounded-md px-4 py-2 text-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all disabled:bg-gray-850 disabled:text-gray-600 disabled:cursor-not-allowed"
          >
            {gameModes.length === 0 ? (
              <option value="">No game modes</option>
            ) : (
              <>
                <option value="">-- Select mode --</option>
                {gameModes.map((mode) => (
                  <option key={mode} value={mode}>
                    {getGameModeDisplay(mode)} ({savesByGameMode[mode].length})
                  </option>
                ))}
              </>
            )}
          </select>
        </div>

        {/* Save Selector */}
        <div className="flex-1">
          <label htmlFor="save-select" className="block text-sm font-medium text-gray-400 mb-2">
            Save Name
          </label>
          <select
            id="save-select"
            value={selectedSave || ""}
            onChange={handleSaveChange}
            disabled={!selectedGameModeKey || savesForSelectedMode.length === 0}
            className="w-full bg-gray-800 border border-gray-700 rounded-md px-4 py-2 text-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all disabled:bg-gray-850 disabled:text-gray-600 disabled:cursor-not-allowed"
          >
            {!selectedGameModeKey || savesForSelectedMode.length === 0 ? (
              <option value="">-- Select save --</option>
            ) : (
              <>
                <option value="">-- Select save --</option>
                {savesForSelectedMode.map((save) => (
                  <option key={save.relative_path} value={save.relative_path}>
                    {save.save_name}
                  </option>
                ))}
              </>
            )}
          </select>
        </div>
      </div>

      {/* Footer info */}
      <p className="mt-3 text-xs text-gray-500 flex items-center justify-between">
        <span>
          {allSaves.length} save{allSaves.length !== 1 ? "s" : ""} across {gameModes.length} game
          mode
          {gameModes.length !== 1 ? "s" : ""}
        </span>
        {selectedSave && (
          <span className="text-gray-400">
            {getGameModeDisplay(selectedGameModeKey)} /{" "}
            {savesForSelectedMode.find((s) => s.relative_path === selectedSave)?.save_name}
          </span>
        )}
      </p>
    </div>
  );
};
