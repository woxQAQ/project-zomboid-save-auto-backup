import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

interface SaveSelectorProps {
  selectedSave: string | null;
  onSaveChange: (saveName: string | null) => void;
}

interface SaveInfo {
  name: string;
  path: string;
}

/**
 * SaveSelector component
 * Displays a dropdown selector for choosing between available saves
 */
export const SaveSelector: React.FC<SaveSelectorProps> = ({ selectedSave, onSaveChange }) => {
  const [saves, setSaves] = useState<SaveInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadSaves = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const saveNames: string[] = await invoke("list_save_directories");
      const saveInfo: SaveInfo[] = saveNames.map((name) => ({
        name,
        path: name, // We'll store the name as both name and path for simplicity
      }));
      setSaves(saveInfo);

      // Auto-select the first save if none is selected
      if (!selectedSave && saveInfo.length > 0) {
        onSaveChange(saveInfo[0].name);
      } else if (selectedSave && !saveNames.includes(selectedSave)) {
        // If the currently selected save no longer exists, clear selection
        onSaveChange(null);
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

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
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

  if (saves.length === 0) {
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
      <label htmlFor="save-select" className="block text-sm font-medium text-gray-400 mb-2">
        Select Save
      </label>
      <select
        id="save-select"
        value={selectedSave || ""}
        onChange={handleChange}
        className="w-full bg-gray-800 border border-gray-700 rounded-md px-4 py-2 text-foreground focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent transition-all"
      >
        <option value="">-- Select a save --</option>
        {saves.map((save) => (
          <option key={save.name} value={save.name}>
            {save.name}
          </option>
        ))}
      </select>
      <p className="mt-2 text-xs text-gray-500">
        {saves.length} save{saves.length !== 1 ? "s" : ""} available
      </p>
    </div>
  );
};
