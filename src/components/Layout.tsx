import React, { ReactNode } from 'react';
import { Header } from './Header';

interface LayoutProps {
  children: ReactNode;
  onSettingsClick?: () => void;
}

/**
 * Main layout component for the application
 * Provides the header and main content area container
 */
export const Layout: React.FC<LayoutProps> = ({ children, onSettingsClick }) => {
  return (
    <div className="h-screen flex flex-col bg-background text-foreground overflow-hidden">
      <Header onSettingsClick={onSettingsClick} />
      <main className="flex-1 overflow-auto">
        {children}
      </main>
    </div>
  );
};
