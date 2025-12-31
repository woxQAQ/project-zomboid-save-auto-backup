import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { Dashboard, Layout, Settings, UpdateAvailableModal } from "./components";

interface UpdateInfo {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  release_url: string;
  release_notes: string;
  published_at: string;
}

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [hasCheckedForUpdates, setHasCheckedForUpdates] = useState(false);

  const handleSettingsClick = useCallback(() => {
    setShowSettings((prev) => !prev);
  }, []);

  const handleCloseSettings = useCallback(() => {
    setShowSettings(false);
  }, []);

  const handleCloseUpdateModal = useCallback(() => {
    setShowUpdateModal(false);
  }, []);

  // Check for updates on startup (if enabled)
  useEffect(() => {
    const checkForUpdatesOnStartup = async () => {
      // Don't check if already checked
      if (hasCheckedForUpdates) {
        return;
      }

      try {
        // Check if auto-update is enabled
        const autoCheck = await invoke<boolean>("get_auto_check_updates");

        if (autoCheck) {
          // Perform update check
          const info: UpdateInfo = await invoke("check_for_updates");

          if (info.has_update) {
            setUpdateInfo(info);
            // Delay showing the modal to let user settle in first
            setTimeout(() => {
              setShowUpdateModal(true);
            }, 5000);
          }
        }
      } catch (err) {
        console.error("Failed to check for updates on startup:", err);
      } finally {
        setHasCheckedForUpdates(true);
      }
    };

    checkForUpdatesOnStartup();
  }, [hasCheckedForUpdates]);

  return (
    <>
      <Layout onSettingsClick={handleSettingsClick}>
        <Dashboard />
      </Layout>

      <Settings isOpen={showSettings} onClose={handleCloseSettings} />

      {updateInfo && (
        <UpdateAvailableModal
          isOpen={showUpdateModal}
          onClose={handleCloseUpdateModal}
          updateInfo={updateInfo}
        />
      )}
    </>
  );
}

export default App;
