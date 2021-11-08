import * as React from 'react';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { CssBaseline } from '@mui/material';
import { getDesignTokens } from './theme';
import { useMainContext } from '../context/main';

/**
 * Provides the theme for the Network Explorer by reacting to the light/dark mode choice stored in the app context.
 */
export const NetworkExplorerThemeProvider: React.FC = ({ children }) => {
  const { mode } = useMainContext();
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode)), [mode]);
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
