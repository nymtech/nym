import React, { useContext } from 'react';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { CssBaseline } from '@mui/material';
import { getDesignTokens } from './theme';
import { AppContext } from '../context/main';
import { NymWalletThemeWithMode } from './NymWalletTheme';

/**
 * Provides the theme for the Network Explorer by reacting to the light/dark mode choice stored in the app context.
 */
export const NymWalletTheme: FCWithChildren = ({ children }) => {
  const { mode } = useContext(AppContext);
  return <NymWalletThemeWithMode mode={mode}>{children}</NymWalletThemeWithMode>;
};

/**
 * Themed component specifically for authentication screens
 */
export const AuthTheme: FCWithChildren = ({ children }) => {
  // Uses dark mode by default for auth screens
  const theme = createTheme(getDesignTokens('dark'));
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
