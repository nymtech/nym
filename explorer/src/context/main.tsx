import * as React from 'react';
import { createContext, useState } from 'react';

export const ExplorerContext = createContext({});

export const ExplorerProvider: React.FC = ({ children }: any) => {
  const [mode, setMode] = useState('light');

  const toggleMode = () => setMode((m) => (m !== 'light' ? 'light' : 'dark'));

  return (
    <ExplorerContext.Provider value={{ mode, toggleMode }}>
      {children}
    </ExplorerContext.Provider>
  );
};
