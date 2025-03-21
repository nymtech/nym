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
  highlight: 'rgb(20, 231, 111)',
  success: 'rgb(20, 231, 111)',
  info: '#60D7EF',
  red: '#E33B5A',
  fee: '#967FF0',
  background: {
    light: '#242B2D',
    dark: '#1C1B1F',
  },
  text: {
    light: '#E6E1E5',
    dark: '#FFFFFF',
    muted: '#938F99',
    grey: '#79747E',
  },
  linkHover: 'rgb(20, 231, 111)',
  border: {
    menu: '#49454F',
  },
};

const darkMode: NymPaletteVariant = {
  mode: 'dark',
  background: {
    main: '#242B2D',
    paper: '#32373D',
    warn: '#F97316',
    grey: '#3A373F',
    greyStroke: '#49454F',
  },
  text: {
    main: '#FFFFFF',
    muted: '#938F99',
    warn: '#F97316',
    contrast: '#242B2D',
    grey: '#79747E',
    blue: '#60D7EF',
  },
  topNav: {
    background: '#1C1B1F',
  },
  nav: {
    background: '#32373D',
  },
  hover: {
    background: '#3A373F',
  },
  modal: {
    border: '#49454F',
  },
  chart: {
    grey: '#49454F',
  },
};

const lightMode: NymPaletteVariant = {
  mode: 'light',
  background: {
    main: '#FFFFFF',
    paper: '#F4F6F8',
    warn: '#F97316',
    grey: '#E2E8EC',
    greyStroke: '#8DA3B1',
  },
  text: {
    main: '#1C1B1F',
    muted: '#79747E',
    warn: '#F97316',
    contrast: '#FFFFFF',
    grey: '#696571',
    blue: '#60D7EF',
  },
  topNav: {
    background: '#FFFFFF',
  },
  nav: {
    background: '#F4F6F8',
  },
  hover: {
    background: '#E2E8EC',
  },
  modal: {
    border: 'transparent',
  },
  chart: {
    grey: '#79747E',
  },
};

const nymWalletPalette = (variant: NymPaletteVariant): NymWalletPalette => ({
  nymWallet: variant,
});

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
  error: {
    main: nymPalette.red,
  },
  background: {
    default: variant.background.main,
    paper: variant.background.paper,
  },
});

const createLightModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPalette(lightMode),
  },
  ...variantToMUIPalette(lightMode),
});

const createDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...nymWalletPalette(darkMode),
  },
  ...variantToMUIPalette(darkMode),
});
export const getDesignTokens = (mode: PaletteMode): ThemeOptions => {
  const { palette } = createTheme({
    palette: {
      mode,
      ...(mode === 'light' ? createLightModePalette() : createDarkModePalette()),
    },
  });

  return {
    typography: {
      fontFamily: ['Lato', 'sans-serif', 'BlinkMacSystemFont', 'Roboto', 'Oxygen', 'Ubuntu', 'Helvetica Neue'].join(
        ',',
      ),
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
                  backgroundColor: 'rgba(112, 117, 255, 0.08) !important',
                },
              },
            },
          },
        },
      },
      MuiMenu: {
        styleOverrides: {
          list: ({ _, theme }) => ({
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
