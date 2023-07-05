import { createTheme, Palette, ThemeOptions } from '@mui/material/styles';
import { PaletteMode } from '@mui/material';
import { nymPalette, variantToMUIPalette, lightMode, darkMode } from './common';

/**
 * Add anything Nym specific to the MUI theme.
 */
// eslint-disable-next-line @typescript-eslint/no-empty-interface
export interface NymTheme {
  palette: Palette;
}

/**
 * Gets the theme options to be passed to `createTheme`.
 *
 * Based on pattern from https://mui.com/customization/dark-mode/#dark-mode-with-custom-palette.
 *
 * @param mode     The theme mode: 'light' or 'dark'
 */
export const getDesignTokens = (mode: PaletteMode): ThemeOptions => {
  // first, create the palette from user's choice of light or dark mode
  const { palette } = createTheme({
    palette: {
      mode,
      ...(mode === 'light' ? variantToMUIPalette(lightMode) : variantToMUIPalette(darkMode)),
    },
  });

  // then customise theme and components
  return {
    typography: {
      fontFamily: [
        'Open Sans',
        'sans-serif',
        'BlinkMacSystemFont',
        'Roboto',
        'Oxygen',
        'Ubuntu',
        'Helvetica Neue',
      ].join(','),
      fontSize: 14,
      fontWeightRegular: 500,
      fontWeightMedium: 600,
      button: {
        textTransform: 'none',
        fontWeight: '600',
      },
    },
    shape: {
      borderRadius: 8,
    },
    transitions: {
      duration: {
        shortest: 150,
        shorter: 200,
        short: 250,
        standard: 300,
        complex: 375,
        enteringScreen: 225,
        leavingScreen: 195,
      },
      easing: {
        easeIn: 'cubic-bezier(0.4, 0, 1, 1)',
      },
    },
    components: {
      MuiButton: {
        styleOverrides: {
          sizeLarge: {
            height: 55,
          },
        },
      },
      MuiStepIcon: {
        styleOverrides: {
          root: {
            '&.Mui-completed': {
              color: nymPalette.status.success,
            },
            '&.Mui-active': {
              color: nymPalette.dark,
            },
          },
        },
      },
      MuiLink: {
        defaultProps: {
          underline: 'none',
        },
      },
    },
    palette,
  };
};
