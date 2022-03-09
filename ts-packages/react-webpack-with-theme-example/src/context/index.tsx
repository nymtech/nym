import { PaletteMode } from '@mui/material';
import * as React from 'react';

interface State {
  mode: PaletteMode;
  toggleMode: () => void;
}

const AppContext = React.createContext<State | undefined>(undefined);

export const useAppContext = (): State => {
  const context = React.useContext<State | undefined>(AppContext);

  if (!context) {
    throw new Error('Please include a `import { AppContextProvider } from "./context"` before using this hook');
  }

  return context;
};

export const AppContextProvider: React.FC = ({ children }) => {
  // light/dark mode
  const [mode, setMode] = React.useState<PaletteMode>('dark');

  const value = React.useMemo<State>(
    () => ({
      mode,
      toggleMode: () => setMode((prevMode) => (prevMode !== 'light' ? 'light' : 'dark')),
    }),
    [mode],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
};
