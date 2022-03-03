import * as React from 'react';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { CssBaseline, PaletteMode } from '@mui/material';
import { getDesignTokens } from './theme';

/**
 * Provides the theme for the Nym Components by reacting to the light/dark mode choice.
 *
 */
export const NymThemeProvider: React.FC<{ mode: PaletteMode }> = ({ mode, children }) => {
  const theme = React.useMemo(() => createTheme(getDesignTokens(mode)), [mode]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};

export type { NymPalette, NymPaletteVariant } from './common';
export type { NymTheme, NymPaletteWithExtensions, NymPaletteWithExtensionsOptions } from './theme';
