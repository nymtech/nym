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
  red: '#DA465B',
  fee: '#967FF0',
  background: { light: '#F4F6F8', dark: '#1D2125' },
  text: {
    light: '#F2F2F2',
    dark: '#121726',
    muted: '#7D7D7D',
    grey: '#5B6174',
  },
  linkHover: '#AF4D36',
  border: {
    menu: '#E8E9EB',
  },
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
    blue: '#60D7EF',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#292E34',
  },
  hover: {
    background: '#36393E',
  },
  modal: {
    border: '#484d53',
  },
  chart: { grey: '#3D4249' },
};

const lightMode: NymPaletteVariant = {
  mode: 'light',
  background: {
    main: '#F4F6F8',
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
    blue: '#514EFB',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#FFFFFF',
  },
  hover: {
    background: '#F9F9F9',
  },
  modal: {
    border: 'transparent',
  },
  chart: { grey: '#E6E6E6' },
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
      MuiToolbar: {
        styleOverrides: {
          root: {
            minWidth: 0,
            '@media (min-width: 0px)': {
              minHeight: 'fit-content',
            },
          },
        },
      },
      MuiSelect: {
        defaultProps: {
          MenuProps: {
            PaperProps: {
              sx: {
                '&& .Mui-selected': {
                  color: nymPalette.highlight,
                  backgroundColor: (t) =>
                    t.palette.mode === 'dark' ? `${t.palette.background.default} !important` : '#FFFFFF !important',
                },
                '&& .Mui-selected:hover': {
                  backgroundColor: 'rgba(251, 110, 78, 0.08) !important',
                },
              },
            },
          },
        },
      },
      MuiMenu: {
        styleOverrides: {
          list: ({ theme }) => ({
            backgroundColor: theme.palette.mode === 'dark' ? darkMode.background.main : undefined,
            border: `1px solid ${theme.palette.nym.border.menu}`,
            borderRadius: '8px',
          }),
        },
      },
    },
    palette,
  };
};
