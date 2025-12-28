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

  return (
    <>
      <Layout onSettingsClick={handleSettingsClick}>
        <Dashboard />
      </Layout>

      <Settings isOpen={showSettings} onClose={handleCloseSettings} />
    </>
  );
}

export default App;
