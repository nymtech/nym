import React, { useMemo, useEffect } from 'react';
import { CssBaseline, PaletteMode } from '@mui/material';
import { createTheme, ThemeProvider } from '@mui/material/styles';
import { getDesignTokens } from './theme';
import '@assets/fonts/non-variable/fonts.css';

let fontsInitialized = false;
let interFontLink: HTMLLinkElement | null = null;

const FontLoader = () => {
  useEffect(() => {
    // Skip if already initialized
    if (fontsInitialized === true) { return; }
    
    fontsInitialized = true;
    
    interFontLink = document.createElement('link');
    interFontLink.rel = 'stylesheet';
    interFontLink.href = 'https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap';
    document.head.appendChild(interFontLink);

    return () => {
      // Only clean up if the component is truly being unmounted
      if (interFontLink && document.head.contains(interFontLink)) {
        document.head.removeChild(interFontLink);
        interFontLink = null;
        fontsInitialized = false;
      }
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