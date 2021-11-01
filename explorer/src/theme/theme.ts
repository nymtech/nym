import { PaletteMode } from '@mui/material';
import {
  PaletteOptions,
  NymPalette,
  NetworkExplorerPalette,
  ThemeOptions,
  createTheme,
} from '@mui/material/styles';

//-----------------------------------------------------------------------------------------------
// Nym palette type definitions
//

/**
 * The Nym palette.
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
const nymPalette: NymPalette = {
  /** emphasises important elements */
  highlight: '#FB6E4E',
};

/**
 * Nym palette specific to the Network Explorer
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
const networkExplorerPalette: NetworkExplorerPalette = {
  networkExplorer: {
    /** world map styles */
    map: {
      background: '#F4F8FA',
      stroke: '#333333',
      fills: ['#EFEFEF', '#FBE7E1', '#F7D1C6', '#F09379'],
    },
    /** left nav styles */
    nav: {
      selected: {
        main: '#111826',
        nested: '#3C4558',
      },
    },
  },
};

//-----------------------------------------------------------------------------------------------
// Nym palettes for light and dark mode
//

/**
 * Returns the Network Explorer palette for light mode.
 */
const createLightModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...networkExplorerPalette,
  },
});

/**
 * Returns the Network Explorer palette for dark mode.
 */
const createDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...networkExplorerPalette,
  },
});

/**
 * IMPORANT: if you need to get the default MUI theme, use the following
 *
 *   import { createTheme as systemCreateTheme } from '@mui/system';
 *
 *   // get the MUI system defaults for light mode
 *   const systemTheme = systemCreateTheme({ palette: { mode: 'light' } });
 *
 *
 *   return {
 *     // change `primary` to default MUI `success`
 *     primary: {
 *       main: systemTheme.palette.success.main,
 *     },
 *     nym: {
 *       ...nymPalette,
 *       ...networkExplorerPalette,
 *     },
 *   };
 */

//-----------------------------------------------------------------------------------------------
// Nym theme overrides
//

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
      ...(mode === 'light'
        ? createLightModePalette()
        : createDarkModePalette()),
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
      fontWeightBold: 600,
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
      MuiCardHeader: {
        styleOverrides: {
          title: {
            fontSize: '16px',
            fontWeight: 'bold',
          },
        },
      },
      MuiDrawer: {
        styleOverrides: {
          paper: {
            background: palette.secondary.dark,
          },
        },
      },
      MuiListItem: {
        styleOverrides: {
          root: {
            background: palette.secondary.dark,
          },
        },
      },
    },
    palette,
  };
};
