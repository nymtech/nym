import { PaletteMode } from '@mui/material';
import {
  createTheme,
  NymMixnetPalette,
  NymPalette,
  NymPaletteVariant,
  PaletteOptions,
  ThemeOptions,
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
  success: '#21D073',
  info: '#60D7EF',
  warning: '#FFE600',
  fee: '#967FF0',
  background: { light: '#F4F6F8', dark: '#1D2125' },
  text: {
    light: '#F2F2F2',
    dark: '#1D2125',
  },
  shipyard: '#817FFA',
};

const darkMode: NymPaletteVariant = {
  mode: 'dark',
  background: {
    main: '#1D2125',
    paper: '#242C3D',
  },
  text: {
    main: '#F2F2F2',
  },
  topNav: {
    background: '#111826',
  },
  shipyard: {
    main: '#817FFA',
  },
};

const lightMode: NymPaletteVariant = {
  mode: 'light',
  background: {
    main: '#F2F2F2',
    paper: '#FFFFFF',
  },
  text: {
    main: '#1D2125',
  },
  topNav: {
    background: '#111826',
  },
  shipyard: {
    main: '#817FFA',
  },
};

/**
 * Nym palette specific to the Nym Mixnode
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
const nymMixnetPalette = (variant: NymPaletteVariant): NymMixnetPalette => ({
  nymMixnet: {},
});

//-----------------------------------------------------------------------------------------------
// Nym palettes for light and dark mode
//

/**
 * Map a Nym palette variant onto the MUI palette
 */
const variantToMUIPalette = (variant: NymPaletteVariant): PaletteOptions => ({
  text: {
    primary: variant.text.main,
  },
  primary: {
    main: nymPalette.highlight,
    contrastText: '#fff',
  },
  secondary: {
    main: variant.mode === 'dark' ? nymPalette.background.light : nymPalette.background.dark,
  },
  success: {
    main: nymPalette.success,
  },
  info: {
    main: nymPalette.info,
  },
  warning: {
    main: nymPalette.warning,
  },
  background: {
    default: variant.background.main,
    paper: variant.background.paper,
  },
});

/**
 * Map a Nym palette variant onto the MUI palette for Shipyard
 */
const variantShipyardToMUIPalette = (variant: NymPaletteVariant): PaletteOptions => ({
  text: {
    primary: variant.text.main,
  },
  primary: {
    main: nymPalette.shipyard,
    contrastText: '#fff',
  },
  secondary: {
    main: variant.mode === 'dark' ? nymPalette.background.light : nymPalette.background.dark,
  },
  success: {
    main: nymPalette.success,
  },
  info: {
    main: nymPalette.info,
  },
  warning: {
    main: nymPalette.warning,
  },
  background: {
    default: variant.background.main,
    paper: variant.background.paper,
  },
});

/**
 * Returns the Network Explorer palette for light mode.
 */
const createLightModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymMixnetPalette(lightMode),
  },
  ...variantToMUIPalette(lightMode),
});

/**
 * Returns the Network Explorer palette for dark mode.
 */
const createDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymMixnetPalette(darkMode),
  },
  ...variantToMUIPalette(darkMode),
});

/**
 * Returns the Shipyard palette for dark mode.
 */
const createShipyardDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymMixnetPalette(darkMode),
  },
  ...variantShipyardToMUIPalette(darkMode),
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
 *       ...nymMixnetPalette,
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
export const getDesignTokens = (mode: PaletteMode, isShipyard: boolean = false): ThemeOptions => {
  let overrides;
  if (isShipyard) {
    overrides = createShipyardDarkModePalette();
  } else {
    overrides = mode === 'light' ? createLightModePalette() : createDarkModePalette();
  }

  // create the palette from user's choice of light or dark mode
  const { palette } = createTheme({
    palette: {
      mode,
      ...overrides,
    },
  });

  // then customise theme and components
  return {
    palette,
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
              color: nymPalette.success,
            },
            '&.Mui-active': {
              color: nymPalette.background.dark,
            },
          },
        },
      },
    },
  };
};
