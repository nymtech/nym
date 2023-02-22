import React from 'react';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { CssBaseline } from '@mui/material';
import { getDesignTokens } from './theme';
// eslint-disable-next-line import/no-relative-packages
import '../../../../assets/fonts/non-variable/fonts.css';

/**
 * Provides the theme for Nym Connect by reacting to the light/dark mode choice stored in the app context.
 */
export const NymMixnetTheme: FCWithChildren<{ mode: 'light' | 'dark' }> = ({ children, mode }) => {
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode)), [mode]);
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};

export const NymShipyardTheme: FCWithChildren<{ mode?: 'light' | 'dark' }> = ({ children, mode = 'dark' }) => {
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode, true)), [mode]);
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
