import { createTheme, Palette, PaletteOptions, ThemeOptions } from '@mui/material/styles';
import { PaletteMode } from '@mui/material';
import { darkMode, lightMode, nymPalette, NymPalette, variantToMUIPalette } from './common';
import { NymWalletPalette, nymWalletPallete } from './wallet';
import { networkExplorerPalette, NetworkExplorerPalette } from './network-explorer';

/**
 * To use the theme, copy the file in `../../template/mui-theme.d.ts` into `src/typings/mui-theme.d.ts` in your project.
 *
 * This will augment the types for `Theme` from `@mui/material/styles` with Nym theme types.
 */

/**
 * "Namespace" in MUI palette for Nym that is a union of the base palette and product palettes
 */
export interface NymPaletteWithExtensions {
  nym: NymPalette & NymWalletPalette & NetworkExplorerPalette;
}

/**
 * Add anything Nym specific to the MUI theme.
 */
// eslint-disable-next-line @typescript-eslint/no-empty-interface
export interface NymTheme {
  palette: Palette & NymPaletteWithExtensions;
}

/**
 * Type use by MUI's `createTheme` method
 */
export type NymPaletteWithExtensionsOptions = Partial<NymPaletteWithExtensions>;

/**
 * Returns the Nym palette for light mode.
 */
export const createLightModePalette = (): PaletteOptions & NymPaletteWithExtensionsOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPallete(lightMode),
    ...networkExplorerPalette(lightMode),
  },
  ...variantToMUIPalette(lightMode),
});

/**
 * Returns the Nym palette for dark mode.
 */
export const createDarkModePalette = (): PaletteOptions & NymPaletteWithExtensionsOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPallete(darkMode),
    ...networkExplorerPalette(darkMode),
  },
  ...variantToMUIPalette(darkMode),
});

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
      ...(mode === 'light' ? createLightModePalette() : createDarkModePalette()),
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
