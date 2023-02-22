import * as React from 'react';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { CssBaseline, PaletteMode } from '@mui/material';
import { getDesignTokens } from './theme';
import { getNetworkExplorerDesignTokens } from './network-explorer';

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

/**
 * Provides the theme with overrides for Network Explorer Components
 *
 * TODO: remove this provider and theme tokens to unify theme
 *
 */
export const NymNetworkExplorerThemeProvider: React.FC<{ mode: PaletteMode }> = ({ mode, children }) => {
  const theme = React.useMemo(() => createTheme(getNetworkExplorerDesignTokens(mode)), [mode]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};

export type { NymPalette, NymPaletteVariant } from './common';
export type { NymTheme, NymPaletteWithExtensions, NymPaletteWithExtensionsOptions } from './theme';
