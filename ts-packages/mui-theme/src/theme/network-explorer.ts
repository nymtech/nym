import { PaletteMode } from '@mui/material';
import { createTheme, ThemeOptions } from '@mui/material/styles';
import { darkMode, nymPalette, NymPaletteVariant } from './common';
import { createDarkModePalette, createLightModePalette } from './theme';

/**
 * A palette definition only for the Network Explorer that extends the Nym palette
 */

export interface NetworkExplorerPalette {
  networkExplorer: {
    map: {
      stroke: string;
      fills: string[];
    };
    background: {
      tertiary: string;
    };
    topNav: {
      background: string;
      socialIcons: string;
      appBar: string;
    };
    nav: {
      selected: {
        main: string;
        nested: string;
      };
      background: string;
      hover: string;
      text: string;
    };
    footer: {
      socialIcons: string;
    };
    mixnodes: {
      status: {
        active: string;
        standby: string;
        inactive: string;
      };
    };
    selectionChance: {
      overModerate: string;
      moderate: string;
      underModerate: string;
    };
  };
}

/**
 * Nym palette specific to the Network Explorer
 *
 * IMPORTANT: do not export this constant, always use the MUI `useTheme` hook to get the correct
 * colours for dark/light mode.
 */
export const networkExplorerPalette = (variant: NymPaletteVariant): NetworkExplorerPalette => ({
  networkExplorer: {
    /** world map styles */
    map: {
      stroke: '#333333',
      fills: ['rgba(255,255,255,0.2)', '#EFEFEF', '#FBE7E1', '#F7D1C6', '#F09379'],
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
      text: variant.nav.text,
    },
    topNav: {
      ...variant.topNav,
      appBar: '#080715',
      socialIcons: '#F2F2F2',
    },
    footer: {
      socialIcons: variant.mode === 'light' ? nymPalette.muted.onDarkBg : darkMode.text.main,
    },
    mixnodes: {
      status: {
        active: variant.mixnodes.status.active,
        standby: variant.mixnodes.status.standby,
        inactive: variant.text.main,
      },
    },
    selectionChance: {
      overModerate: variant.selectionChance.overModerate,
      moderate: variant.selectionChance.moderate,
      underModerate: variant.selectionChance.underModerate,
    },
  },
});

/**
 * Gets the theme options to be passed to `createTheme` for Network Explorer.
 *
 * Based on pattern from https://mui.com/customization/dark-mode/#dark-mode-with-custom-palette.
 *
 * TODO: remove this by unifying theme
 *
 * @param mode     The theme mode: 'light' or 'dark'
 */
export const getNetworkExplorerDesignTokens = (mode: PaletteMode): ThemeOptions => {
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
            fontSize: 16,
            fontWeight: 600,
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
      MuiPaper: {
        styleOverrides: {
          root: {
            borderRadius: '10px',
          },
          elevation1: {
            backgroundImage: mode === 'dark' ? 'none' : undefined,
          },
          elevation2: {
            backgroundImage: mode === 'dark' ? 'none' : undefined,
          },
        },
      },
    },
    palette,
  };
};
