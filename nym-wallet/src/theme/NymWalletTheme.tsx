import React, { useMemo, useEffect } from 'react';
import { CssBaseline, PaletteMode } from '@mui/material';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { getDesignTokens } from './theme';
import '@assets/fonts/non-variable/fonts.css';

// Component that applies the Inter font for better typography
const FontLoader = () => {
  useEffect(() => {
    // Add Inter font for modern typography
    const interFontLink = document.createElement('link');
    interFontLink.rel = 'stylesheet';
    interFontLink.href = 'https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap';
    document.head.appendChild(interFontLink);

    return () => {
      // Clean up
      document.head.removeChild(interFontLink);
    };
  }, []);

  return null;
};

export const NymWalletThemeWithMode: FCWithChildren<{ mode: PaletteMode; children: React.ReactNode }> = ({
  mode,
  children,
}) => {
  // Create theme with memoization for performance
  const theme = useMemo(() => createTheme(getDesignTokens(mode)), [mode]);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <FontLoader />
      {children}
    </ThemeProvider>
  );
};
