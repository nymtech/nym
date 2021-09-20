import * as React from 'react';
import { createContext, useState } from 'react';

export const DarkModeContext = createContext({});

export const DarkModeProvider: React.FC = ({ children }: any) => {
  const [mode, setMode] = useState('light');

  const toggleMode = () => setMode((m) => (m !== 'light' ? 'light' : 'dark'));

  return (
    <DarkModeContext.Provider value={{ mode, toggleMode }}>
      {children}
    </DarkModeContext.Provider>
  );
};
