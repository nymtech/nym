import React from 'react';
import { CssBaseline, PaletteMode } from '@mui/material';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { getDesignTokens } from './theme';
import '@assets/fonts/non-variable/fonts.css';

export const NymWalletThemeWithMode: FCWithChildren<{ mode: PaletteMode; children: React.ReactNode }> = ({
  mode,
  children,
}) => {
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode)), [mode]);
  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
