import React, { useContext } from 'react';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { CssBaseline } from '@mui/material';
import { getDesignTokens } from './theme';
import { ClientContext } from '../context/main';
import '../../../assets/fonts/non-variable/fonts.css';

/**
 * Provides the theme for Nym Connect by reacting to the light/dark mode choice stored in the app context.
 */
export const NymMixnetTheme: React.FC<{ mode: 'light' | 'dark' }> = ({ children, mode }) => {
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode)), [mode]);
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
