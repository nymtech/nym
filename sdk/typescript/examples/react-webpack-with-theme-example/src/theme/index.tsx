import * as React from 'react';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { CssBaseline, PaletteMode } from '@mui/material';
import { getDesignTokens } from './theme';

export const NymThemeProvider: FCWithChildren<{ mode: PaletteMode; children: React.ReactNode }> = ({
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
