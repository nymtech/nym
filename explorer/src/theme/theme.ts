import { PaletteMode } from '@mui/material';
import {
  PaletteOptions,
  NymPalette,
  NetworkExplorerPalette,
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
  text: {
    nav: '#F2F2F2',
    /** footer text colour */
    footer: '#666B77',
  },
  mixnodes: {
    status: {
      active: '#20D073',
      standby: '#5FD7EF',
    },
  },
};

const darkMode: NymPaletteVariant = {
  mode: 'dark',
  background: {
    main: '#111826',
    paper: '#242C3D',
  },
  text: {
    main: '#F2F2F2',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#242C3D',
    hover: '#111826',
  },
};

const lightMode: NymPaletteVariant = {
  mode: 'light',
  background: {
    main: '#F2F2F2',
    paper: '#FFFFFF',
  },
  text: {
    main: '#666666',
  },
  topNav: {
    background: '#111826',
  },
  nav: {
    background: '#242C3D',
    hover: '#111826',
  },
};

/**
 * Nym palette specific to the Network Explorer
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
const networkExplorerPalette = (
  variant: NymPaletteVariant,
): NetworkExplorerPalette => ({
  networkExplorer: {
    /** world map styles */
    map: {
      stroke: '#333333',
      fills: ['#EFEFEF', '#FBE7E1', '#F7D1C6', '#F09379'],
    },
    background: {
      tertiary: variant.mode === 'light' ? '#F4F8FA' : '#323C51',
    },
    /** left nav styles */
    nav: {
      selected: {
        main: '#111826',
        nested: '#3C4558',
      },
      background: variant.nav.background,
      hover: variant.nav.hover,
      text: nymPalette.text.nav,
    },
    topNav: {
      ...variant.topNav,
      appBar: '#070B15',
      socialIcons: '#F2F2F2',
    },
    footer: {
      socialIcons:
        variant.mode === 'light' ? nymPalette.text.footer : darkMode.text.main,
    },
    mixnodes: {
      status: {
        active: nymPalette.mixnodes.status.active,
        standby: nymPalette.mixnodes.status.standby,
        inactive: variant.text.main,
      },
    },
  },
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
    ...networkExplorerPalette(lightMode),
  },
  ...variantToMUIPalette(lightMode),
});

/**
 * Returns the Network Explorer palette for dark mode.
 */
const createDarkModePalette = (): PaletteOptions => ({
  nym: {
    ...nymPalette,
    ...networkExplorerPalette(darkMode),
  },
  ...variantToMUIPalette(darkMode),
});

/**
 * IMPORTANT: if you need to get the default MUI theme, use the following
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
      fontWeightRegular: 600,
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
            fontSize: 18,
            fontWeight: 800,
          },
        },
      },
      MuiDrawer: {
        styleOverrides: {
          paper: {
            background: palette.secondary.dark,
            marginTop: 64,
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
