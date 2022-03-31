import React from 'react';
import { CssBaseline, PaletteMode } from '@mui/material';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { getDesignTokens } from './theme';
import '@assets/fonts/fonts.css';

export const NymWalletThemeWithMode: React.FC<{ mode: PaletteMode }> = ({ mode, children }) => {
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode)), [mode]);
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
