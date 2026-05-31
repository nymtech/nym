import React from 'react';
import type { Preview } from '@storybook/react-webpack5';
import { CacheProvider } from '@emotion/react';
import { CssBaseline } from '@mui/material';
import { ThemeProvider, createTheme } from '@mui/material/styles';
import { SnackbarProvider } from 'notistack';
import { getDesignTokens } from '../src/theme/theme';
import { muiEmotionCache } from '../src/theme/emotionCache';

// Build the theme directly from design tokens + emotion cache. We deliberately
// avoid `NymWalletTheme`, which imports `@assets/...fonts.css` (a tsconfig path
// alias Storybook's webpack can't resolve). Fonts fall back to system defaults
// in Storybook, which is fine for component/flow rendering.
const storybookTheme = createTheme(getDesignTokens('dark'));

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
  decorators: [
    (Story) => (
      <CacheProvider value={muiEmotionCache}>
        <ThemeProvider theme={storybookTheme}>
          <CssBaseline />
          <SnackbarProvider anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}>
            <Story />
          </SnackbarProvider>
        </ThemeProvider>
      </CacheProvider>
    ),
  ],
};

export default preview;
