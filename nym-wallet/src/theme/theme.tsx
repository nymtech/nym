import { PaletteMode } from '@mui/material';
import {
  PaletteOptions,
  NymPalette,
  NymWalletPalette,
  ThemeOptions,
  createTheme,
  NymPaletteVariant,
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
  fee: '#967FF0',
  background: { light: '#F4F6F8', dark: '#1D2125' },
  text: {
    light: '#F2F2F2',
    dark: '#121726',
    muted: '#7D7D7D',
    grey: '#5B6174',
  },
  linkHover: '#AF4D36',
};

const darkMode: NymPaletteVariant = {
  mode: 'dark',
  background: {
    main: '#1D2125',
    paper: '#292E34',
    warn: '#FFE600',
    grey: '#3A4053',
    greyStroke: '#545D6A',
  },
  text: {
    main: '#FFFFFF',
    muted: '#7D7D7D',
    warn: '#FFE600',
    contrast: '#1D2125',
    grey: '#5B6174',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#292E34',
  },
  mixnodes: {
    status: {
      active: '#20D073',
      standby: '#5FD7EF',
    },
  },
  selectionChance: {
    overModerate: '#20D073',
    moderate: '#EBA53D',
    underModerate: '#DA465B',
  },
};

const lightMode: NymPaletteVariant = {
  mode: 'light',
  background: {
    main: '#E5E5E5',
    paper: '#FFFFFF',
    warn: '#FFE600',
    grey: '#F5F5F5',
    greyStroke: '#E6E6E6',
  },
  text: {
    main: '#121726',
    muted: '#7D7D7D',
    warn: '#FFE600',
    contrast: '#FFFFFF',
    grey: '#3A4053',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#FFFFFF',
  },
  mixnodes: {
    status: {
      active: '#1CBB67',
      standby: '#55C1D7',
    },
  },
  selectionChance: {
    overModerate: '#20D073',
    moderate: '#EBA53D',
    underModerate: '#DA465B',
  },
};

/**
 * Nym palette specific to the Nym Wallet
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
const nymWalletPalette = (variant: NymPaletteVariant): NymWalletPalette => ({
  nymWallet: variant,
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
    disabled: variant.text.grey,
  },
  primary: {
    main: nymPalette.highlight,
    contrastText: variant.text.contrast,
  },
  secondary: {
    main: variant.text.main,
  },
  success: {
    main: nymPalette.success,
  },
  info: {
    main: nymPalette.info,
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
    ...nymWalletPalette(lightMode),
  },
  ...variantToMUIPalette(lightMode),
});

/**
 * Returns the Network Explorer palette for dark mode.
 */
const createDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPalette(darkMode),
  },
  ...variantToMUIPalette(darkMode),
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
 *       ...nymWalletPalette,
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
      fontWeightRegular: 400,
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
      MuiTypography: {
        styleOverrides: {
          root: {
            fontSize: 14,
          },
        },
      },
      MuiButton: {
        styleOverrides: {
          root: {
            fontSize: 16,
          },
          sizeLarge: {
            height: 55,
          },
          outlined: {
            borderWidth: '2px',
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
      MuiTableCell: {
        styleOverrides: {
          root: {
            padding: 0,
            paddingTop: '16px',
            paddingBottom: '16px',
          },
          head: {
            fontWeight: '400',
            color: nymPalette.text.muted,
          },
        },
      },
      MuiLink: {
        defaultProps: {
          underline: 'none',
        },
      },
      MuiDialogTitle: {
        styleOverrides: {
          root: {
            fontWeight: 600,
          },
        },
      },
    },
    palette,
  };
};
