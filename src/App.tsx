import { useState } from "react";
import { Dashboard, Layout, Settings } from "./components";

function App() {
  const [showSettings, setShowSettings] = useState(false);

  const handleSettingsClick = () => {
    setShowSettings(!showSettings);
  };

  const handleCloseSettings = () => {
    setShowSettings(false);
  };

  const handleRestoreInitiate = (saveName: string, backupName: string) => {
    // This will be implemented in UI-04 with the restore confirmation dialog
    console.log("Restore initiated:", { saveName, backupName });
    // TODO: Show confirmation dialog in UI-04
  };

  return (
    <>
      <Layout onSettingsClick={handleSettingsClick}>
        <Dashboard onRestoreInitiate={handleRestoreInitiate} />
      </Layout>

      <Settings isOpen={showSettings} onClose={handleCloseSettings} />
    </>
  );
}

export default App;
