import { useState } from 'react';
import { Layout } from './components';

function App() {
  const [showSettings, setShowSettings] = useState(false);

  const handleSettingsClick = () => {
    setShowSettings(!showSettings);
  };

  return (
    <Layout onSettingsClick={handleSettingsClick}>
      <div className="h-full flex flex-col items-center justify-center p-8">
        <div className="text-center max-w-md">
          {/* Placeholder content */}
          <div className="mb-8">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="w-24 h-24 mx-auto text-primary mb-4"
            >
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.41 0-8-3.59-8-8s3.59-8 8-8 8 3.59 8 8-3.59 8-8 8z" />
              <path d="M10 10h.01M14 10h.01M12 14c-1.1 0-2-.9-2-2h4c0 1.1-.9 2-2 2z" />
              <path d="M9 18v-2h6v2" />
            </svg>
            <h2 className="text-2xl font-bold mb-2">Welcome to PZ Backup Tool</h2>
            <p className="text-gray-400">
              Your Project Zomboid save game backup and restore solution
            </p>
          </div>

          <div className="bg-gray-900 border border-gray-800 rounded-lg p-6 text-left">
            <h3 className="text-lg font-semibold mb-4 text-primary">Getting Started</h3>
            <ul className="space-y-2 text-sm text-gray-300">
              <li className="flex items-start">
                <span className="text-success mr-2">✓</span>
                <span>Configure your save paths in settings</span>
              </li>
              <li className="flex items-start">
                <span className="text-gray-600 mr-2">○</span>
                <span>Select a save to manage</span>
              </li>
              <li className="flex items-start">
                <span className="text-gray-600 mr-2">○</span>
                <span>Create manual backups or enable auto-backup</span>
              </li>
              <li className="flex items-start">
                <span className="text-gray-600 mr-2">○</span>
                <span>Restore from backup history when needed</span>
              </li>
            </ul>
          </div>

          <p className="mt-6 text-sm text-gray-500">
            Click the settings icon in the top right to get started
          </p>
        </div>
      </div>
    </Layout>
  );
}

export default App;
